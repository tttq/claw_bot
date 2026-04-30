// Claw Desktop - 微信加密 - 消息加解密
use aes::Aes128;
use aes::cipher::{BlockDecrypt, BlockEncrypt, KeyInit, generic_array::GenericArray};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};

pub fn aes128_ecb_encrypt(key: &[u8], plaintext: &[u8]) -> Vec<u8> {
    let cipher = Aes128::new(GenericArray::from_slice(key));
    let padded = pkcs7_pad(plaintext, 16);
    let mut result = Vec::with_capacity(padded.len());
    for chunk in padded.chunks(16) {
        let mut block = GenericArray::clone_from_slice(chunk);
        cipher.encrypt_block(&mut block);
        result.extend_from_slice(&block);
    }
    result
}

pub fn aes128_ecb_decrypt(key: &[u8], ciphertext: &[u8]) -> Vec<u8> {
    let cipher = Aes128::new(GenericArray::from_slice(key));
    let mut result = Vec::with_capacity(ciphertext.len());
    for chunk in ciphertext.chunks(16) {
        let mut block = GenericArray::clone_from_slice(chunk);
        cipher.decrypt_block(&mut block);
        result.extend_from_slice(&block);
    }
    pkcs7_unpad(&result)
}

pub fn pkcs7_pad(data: &[u8], block_size: usize) -> Vec<u8> {
    let padding_len = block_size - (data.len() % block_size);
    let mut padded = data.to_vec();
    padded.extend(std::iter::repeat(padding_len as u8).take(padding_len));
    padded
}

pub fn pkcs7_unpad(data: &[u8]) -> Vec<u8> {
    if data.is_empty() {
        return data.to_vec();
    }
    let padding_len = match data.last() {
        Some(&len) => len as usize,
        None => return data.to_vec(),
    };
    if padding_len == 0 || padding_len > data.len() || padding_len > 16 {
        return data.to_vec();
    }
    if data
        .iter()
        .rev()
        .take(padding_len)
        .any(|&b| b as usize != padding_len)
    {
        return data.to_vec();
    }
    data[..data.len() - padding_len].to_vec()
}

pub fn parse_aes_key(raw: &str) -> Option<Vec<u8>> {
    if let Ok(bytes) = BASE64.decode(raw) {
        if bytes.len() == 16 {
            return Some(bytes);
        }
        if let Ok(hex_str) = String::from_utf8(bytes) {
            if hex_str.len() == 32 {
                if let Ok(key_bytes) = hex::decode(&hex_str) {
                    if key_bytes.len() == 16 {
                        return Some(key_bytes);
                    }
                }
            }
        }
    }
    None
}

pub fn encode_aes_key_for_send(aes_key: &[u8]) -> String {
    let hex_str = hex::encode(aes_key);
    BASE64.encode(hex_str.as_bytes())
}

pub fn generate_random_key() -> Vec<u8> {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..16).map(|_| rng.r#gen::<u8>()).collect()
}

pub fn generate_random_filekey() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..32)
        .map(|_| format!("{:02x}", rng.r#gen::<u8>()))
        .collect()
}
