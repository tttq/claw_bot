// Claw Desktop - 渠道加密 - 消息加密和解密
use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rand::RngCore;
use std::collections::HashMap;

const NONCE_SIZE: usize = 12;
const KEY_SIZE: usize = 32;

pub struct EncryptionService {
    cipher: Aes256Gcm,
}

impl EncryptionService {
    pub fn new(master_key: &[u8; KEY_SIZE]) -> Self {
        let cipher = Aes256Gcm::new_from_slice(master_key)
            .expect("Invalid key length");
        Self { cipher }
    }

    pub fn generate_key() -> [u8; KEY_SIZE] {
        let mut key = [0u8; KEY_SIZE];
        OsRng.fill_bytes(&mut key);
        key
    }

    pub fn encrypt_field(&self, plaintext: &str) -> Result<String, String> {
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        OsRng.fill_bytes(&mut nonce_bytes);

        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = self
            .cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| format!("Encryption failed: {}", e))?;

        let mut result = nonce_bytes.to_vec();
        result.extend_from_slice(&ciphertext);

        Ok(BASE64.encode(&result))
    }

    pub fn decrypt_field(&self, encoded: &str) -> Result<String, String> {
        let data = BASE64
            .decode(encoded)
            .map_err(|e| format!("Base64 decode failed: {}", e))?;

        if data.len() < NONCE_SIZE {
            return Err("Invalid encrypted data".to_string());
        }

        let (nonce_bytes, ciphertext) = data.split_at(NONCE_SIZE);
        let nonce = Nonce::from_slice(nonce_bytes);

        let plaintext = self
            .cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| format!("Decryption failed: {}", e))?;

        String::from_utf8(plaintext).map_err(|e| format!("UTF-8 decode failed: {}", e))
    }

    pub fn encrypt_config_fields(
        &self,
        config: &HashMap<String, String>,
        sensitive_keys: &[&str],
    ) -> Result<HashMap<String, String>, String> {
        let mut encrypted = HashMap::new();

        for (key, value) in config {
            if sensitive_keys.contains(&key.as_str()) {
                if !value.is_empty() {
                    encrypted.insert(key.clone(), self.encrypt_field(value)?);
                } else {
                    encrypted.insert(key.clone(), value.clone());
                }
            } else {
                encrypted.insert(key.clone(), value.clone());
            }
        }

        Ok(encrypted)
    }

    pub fn decrypt_config_fields(
        &self,
        config: &HashMap<String, String>,
        sensitive_keys: &[&str],
    ) -> Result<HashMap<String, String>, String> {
        let mut decrypted = HashMap::new();

        for (key, value) in config {
            if sensitive_keys.contains(&key.as_str()) && !value.is_empty() {
                match self.decrypt_field(value) {
                    Ok(plain) => decrypted.insert(key.clone(), plain),
                    Err(_) => decrypted.insert(key.clone(), value.clone()),
                };
            } else {
                decrypted.insert(key.clone(), value.clone());
            }
        }

        Ok(decrypted)
    }

    pub fn mask_sensitive_value(value: &str, visible_chars: usize) -> String {
        if value.len() <= visible_chars * 2 {
            "*".repeat(value.len())
        } else {
            format!(
                "{}{}{}",
                &value[..visible_chars],
                "*".repeat(value.len() - visible_chars * 2),
                &value[value.len() - visible_chars..]
            )
        }
    }
}

pub static SENSITIVE_KEYS: [&str; 4] = ["bot_token", "app_token", "api_key", "webhook_secret"];

pub fn get_sensitive_keys_for_channel(channel_type: &str) -> Vec<&'static str> {
    match channel_type {
        "telegram" => vec!["bot_token"],
        "discord" => vec!["bot_token"],
        "slack" => vec!["bot_token", "app_token"],
        "whatsapp" => vec!["api_key"],
        _ => SENSITIVE_KEYS.to_vec(),
    }
}
