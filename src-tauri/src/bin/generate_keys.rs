// Claw Desktop - RSA密钥生成工具 - 生成服务端RSA密钥对并输出PEM格式
use base64::Engine;
use base64::engine::general_purpose::STANDARD;

fn main() {
    env_logger::init();

    log::info!("[GenerateKeys] ========================================");
    log::info!("[GenerateKeys] Generating RSA keypair for qclaw-desktop...");

    use rsa::pkcs1::EncodeRsaPublicKey;
    use rsa::pkcs8::EncodePrivateKey;
    let mut rng = rand::thread_rng();
    
    let private_key = rsa::RsaPrivateKey::new(&mut rng, 2048)
        .map_err(|e| format!("[GenerateKeys] Failed to generate RSA private key: {}", e))
        .expect("Failed to generate RSA private key");
    
    let private_pem = private_key
        .to_pkcs8_pem(rsa::pkcs8::LineEnding::default())
        .map_err(|e| format!("[GenerateKeys] Failed to encode private key: {}", e))
        .expect("Failed to encode private key")
        .to_string();
        
    let public_key = private_key.to_public_key();
    let public_pem = public_key
        .to_pkcs1_pem(rsa::pkcs8::LineEnding::default())
        .map_err(|e| format!("[GenerateKeys] Failed to encode public key: {}", e))
        .expect("Failed to encode public key")
        .to_string();
    
    let der_bytes = public_key.to_pkcs1_der()
        .map_err(|e| format!("[GenerateKeys] Failed to get DER: {}", e))
        .expect("Failed to get DER")
        .as_ref()
        .to_vec();
    let digest = ring::digest::digest(&ring::digest::SHA256, &der_bytes);
    let fingerprint = hex::encode(&digest.as_ref()[..16]);
    
    log::info!("[GenerateKeys] Key pair generated!");
    log::info!("[GenerateKeys]   Fingerprint: {}", fingerprint);
    log::info!("[GenerateKeys]   Public key (first 40 chars): {}...", &public_pem[..40.min(public_pem.len())]);
    
    let project_root = std::env::current_dir()
        .map_err(|e| format!("[GenerateKeys] Failed to get current dir: {}", e))
        .unwrap()
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().unwrap());
    
    let keys_dir = project_root.join("src-tauri/keys");
    let private_path = keys_dir.join("private.pem");
    let public_ts_path = project_root.join("src/ws/publicKey.ts");
    
    std::fs::create_dir_all(&keys_dir)
        .map_err(|e| format!("[GenerateKeys] Failed to create keys directory: {}", e))
        .expect("Failed to create keys directory");
    
    std::fs::write(&private_path, &private_pem)
        .map_err(|e| format!("[GenerateKeys] Failed to write private key: {}", e))
        .expect("Failed to write private key");
    log::info!("[GenerateKeys] Private key saved to: {:?}", private_path);

    let b64_encoded = STANDARD.encode(public_pem.as_bytes());
    
    let ts_content = format!(
r#"/**
 * Embedded RSA public key - auto-generated file, do not modify manually!
 *
 * Generate command: cargo run --bin generate_keys
 * Updated: {}
 * Key fingerprint: {}
 */

const EMBEDDED_KEY_B64 = '{}' 

export function getEmbeddedPublicKey(): string {{
  try {{
    const decoded = decodeURIComponent(escape(atob(EMBEDDED_KEY_B64)))
    if (!decoded.includes('-----BEGIN')) {{
      throw new Error('[Security] Embedded public key is corrupted')
    }}
    return decoded
  }} catch (e) {{
    throw new Error('[Security] Embedded public key is corrupted: ' + (e as Error).message)
  }}
}}

export function getEnvPublicKey(): string {{
  const envKey = (import.meta as any).env?.VITE_PUBLIC_KEY
  if (envKey?.includes('-----BEGIN')) return envKey
  return ''
}}

export function getDefaultPublicKey(): string {{
  return getEmbeddedPublicKey() || getEnvPublicKey()
}}
"#,
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        fingerprint,
        b64_encoded
    );
    
    std::fs::write(&public_ts_path, ts_content)
        .map_err(|e| format!("[GenerateKeys] Failed to write public key TS file: {}", e))
        .expect("Failed to write public key TS file");
    log::info!("[GenerateKeys] Public key (base64) saved to: {:?}", public_ts_path);
    
    let gitignore_path = keys_dir.join(".gitignore");
    if !gitignore_path.exists() {
        std::fs::write(&gitignore_path, "# Private keys - DO NOT COMMIT\nprivate.pem\n*.key\n")
            .map_err(|e| format!("[GenerateKeys] Failed to write .gitignore: {}", e))
            .expect("Failed to write .gitignore");
        log::info!("[GenerateKeys] Created .gitignore for keys directory");
    }
    
    log::info!("[GenerateKeys] ========================================");
    log::info!("[GenerateKeys] Done! You can now:");
    log::info!("[GenerateKeys]   1. Restart the frontend dev server");
    log::info!("[GenerateKeys]   2. The embedded public key will be used for auto-authentication");
}
