use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use rand::RngCore;

use crate::error::{AppError, AppResult};

const NONCE_SIZE: usize = 12;
const KEY_SIZE: usize = 32;

#[derive(Clone)]
pub struct TokenCrypto {
    cipher: Aes256Gcm,
}

impl TokenCrypto {
    pub fn new() -> AppResult<Self> {
        let key = Self::get_or_create_key()?;
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| AppError::Crypto(e.to_string()))?;
        Ok(Self { cipher })
    }

    fn get_or_create_key() -> AppResult<[u8; KEY_SIZE]> {
        // For now, generate a random key each time
        // TODO: Store in OS keychain or derive from machine-specific data
        let mut key = [0u8; KEY_SIZE];
        rand::rng().fill_bytes(&mut key);
        Ok(key)
    }

    pub fn encrypt(&self, plaintext: &str) -> AppResult<Vec<u8>> {
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        rand::rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = self
            .cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| AppError::Crypto(e.to_string()))?;

        // Prepend nonce to ciphertext
        let mut result = nonce_bytes.to_vec();
        result.extend(ciphertext);
        Ok(result)
    }

    pub fn decrypt(&self, encrypted: &[u8]) -> AppResult<String> {
        if encrypted.len() < NONCE_SIZE {
            return Err(AppError::Crypto("Invalid encrypted data".to_string()));
        }

        let (nonce_bytes, ciphertext) = encrypted.split_at(NONCE_SIZE);
        let nonce = Nonce::from_slice(nonce_bytes);

        let plaintext = self
            .cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| AppError::Crypto(e.to_string()))?;

        String::from_utf8(plaintext).map_err(|e| AppError::Crypto(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let crypto = TokenCrypto::new().unwrap();
        let plaintext = "test-token-12345";

        let encrypted = crypto.encrypt(plaintext).unwrap();
        let decrypted = crypto.decrypt(&encrypted).unwrap();

        assert_eq!(plaintext, decrypted);
    }
}
