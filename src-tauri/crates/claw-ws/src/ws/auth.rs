// Claw Desktop - WS认证 - RSA握手、令牌验证、会话密钥解密
use base64::Engine;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use rsa::pkcs8::{DecodePrivateKey, EncodePrivateKey, EncodePublicKey};
use rsa::sha2::Sha256;
use rsa::{Oaep, RsaPrivateKey, RsaPublicKey};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::path::PathBuf;
use std::sync::OnceLock;

static KEY_PAIR: OnceLock<(RsaPrivateKey, RsaPublicKey)> = OnceLock::new();
static JWT_SECRET: OnceLock<String> = OnceLock::new();

/// 获取应用数据目录
fn get_app_data_dir() -> Option<PathBuf> {
    Some(claw_config::path_resolver::get_app_root().clone())
}

/// 获取JWT密钥文件路径
fn get_secret_file_path() -> Option<PathBuf> {
    Some(claw_config::path_resolver::jwt_secret_path())
}

/// 加载或生成JWT密钥 — 优先从文件加载，不存在则生成新的并持久化
fn load_or_generate_secret() -> String {
    if let Some(path) = get_secret_file_path() {
        if let Ok(secret) = std::fs::read_to_string(&path) {
            let secret = secret.trim().to_string();
            if !secret.is_empty() && secret.len() >= 32 {
                log::info!("[Auth] Loaded persisted JWT secret from {:?}", path);
                return secret;
            }
        }
    }

    log::info!("[Auth] Generating new JWT secret");
    let secret = format!("{:x}", Sha256::digest(&rand::random::<[u8; 32]>()));

    if let Some(dir) = get_app_data_dir() {
        let _ = std::fs::create_dir_all(&dir);
        if let Some(path) = get_secret_file_path() {
            match std::fs::write(&path, &secret) {
                Ok(_) => log::info!("[Auth] Persisted JWT secret to {:?}", path),
                Err(e) => log::warn!("[Auth] Failed to persist secret: {}", e),
            }
        }
    }

    secret
}

/// 获取RSA密钥对文件路径
fn get_keypair_file_path() -> Option<PathBuf> {
    Some(claw_config::path_resolver::rsa_keypair_path())
}

/// 加载或生成RSA密钥对 — 优先从文件加载，不存在则生成2048位RSA密钥并持久化
fn load_or_generate_keypair() -> (RsaPrivateKey, RsaPublicKey) {
    if let Some(path) = get_keypair_file_path() {
        if let Ok(pem_str) = std::fs::read_to_string(&path) {
            match rsa::RsaPrivateKey::from_pkcs8_pem(&pem_str) {
                Ok(private_key) => {
                    let public_key = RsaPublicKey::from(&private_key);
                    log::info!("[Auth] Loaded persisted RSA key pair from {:?}", path);
                    return (private_key, public_key);
                }
                Err(e) => log::warn!(
                    "[Auth] Failed to parse persisted key pair: {}, regenerating...",
                    e
                ),
            }
        }
    }

    log::info!("[Auth] Generating new RSA key pair");
    let mut rng = rand::thread_rng();
    let private_key = RsaPrivateKey::new(&mut rng, 2048).expect("Failed to generate RSA key pair");
    let public_key = RsaPublicKey::from(&private_key);

    if let Some(dir) = get_app_data_dir() {
        let _ = std::fs::create_dir_all(&dir);
        if let Some(path) = get_keypair_file_path() {
            match private_key.to_pkcs8_pem(rsa::pkcs8::LineEnding::LF) {
                Ok(pem) => match std::fs::write(&path, &pem) {
                    Ok(_) => log::info!("[Auth] Persisted RSA key pair to {:?}", path),
                    Err(e) => log::warn!("[Auth] Failed to persist key pair: {}", e),
                },
                Err(e) => log::warn!("[Auth] Failed to serialize key pair: {}", e),
            }
        }
    }

    (private_key, public_key)
}

/// 获取或初始化全局RSA密钥对
fn get_or_init_key_pair() -> &'static (RsaPrivateKey, RsaPublicKey) {
    KEY_PAIR.get_or_init(load_or_generate_keypair)
}

/// 获取JWT密钥
fn get_jwt_secret() -> &'static str {
    JWT_SECRET.get_or_init(|| load_or_generate_secret())
}

/// 获取RSA公钥PEM — 用于客户端加密会话密钥
pub fn get_public_key_pem() -> Result<String, String> {
    let public_key = &get_or_init_key_pair().1;
    let pem = public_key
        .to_public_key_pem(rsa::pkcs8::LineEnding::LF)
        .map_err(|e| format!("Failed to encode public key as SPKI PEM: {}", e))?;
    Ok(pem.into())
}

/// 使用私钥解密 — 解密客户端用公钥加密的会话密钥
pub fn decrypt_with_private_key(encrypted_b64: &str) -> Result<Vec<u8>, String> {
    let private_key = &get_or_init_key_pair().0;
    let encrypted = base64::engine::general_purpose::STANDARD
        .decode(encrypted_b64)
        .map_err(|e| format!("Base64 decode failed: {}", e))?;
    private_key
        .decrypt(Oaep::new::<Sha256>(), &encrypted)
        .map_err(|e| format!("Decryption failed: {}", e))
}

/// JWT Claims结构 — 包含客户端ID、签发时间、过期时间、唯一ID
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub iat: i64,
    pub exp: i64,
    pub jti: String,
    pub client_id: String,
}

/// 生成JWT令牌 — 有效期24小时，返回(token, expires_at)
pub fn generate_token(client_id: &str) -> Result<(String, i64), String> {
    let now = chrono::Utc::now();
    let expires_at = now + chrono::Duration::hours(24);
    let claims = Claims {
        sub: "ws_client".to_string(),
        iat: now.timestamp(),
        exp: expires_at.timestamp(),
        jti: uuid::Uuid::new_v4().to_string(),
        client_id: client_id.to_string(),
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(get_jwt_secret().as_bytes()),
    )
    .map_err(|e| format!("Token generation failed: {}", e))?;

    Ok((token, expires_at.timestamp()))
}

/// 验证JWT令牌 — 解码并校验签名和过期时间，返回Claims
pub fn validate_token(token: &str) -> Result<Claims, String> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(get_jwt_secret().as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| format!("Token validation failed: {}", e))?;

    Ok(token_data.claims)
}

/// 快速检查令牌是否有效
pub fn is_token_valid(token: &str) -> bool {
    validate_token(token).is_ok()
}

/// RSA握手 — 解密客户端发送的加密会话密钥，生成JWT令牌
pub fn handshake(encrypted_session_key_b64: &str) -> Result<(String, i64), String> {
    let decrypted = decrypt_with_private_key(encrypted_session_key_b64)?;
    if decrypted.len() < 16 {
        return Err("Invalid session key: too short".to_string());
    }
    let client_id = format!("{:x}", sha2::Sha256::digest(&decrypted));
    generate_token(&client_id)
}
