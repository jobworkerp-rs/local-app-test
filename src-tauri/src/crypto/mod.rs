use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use keyring::Entry;
use rand::RngCore;

use crate::error::{AppError, AppResult};

const NONCE_SIZE: usize = 12;
const KEY_SIZE: usize = 32;
const KEYRING_SERVICE: &str = "local-code-agent";
const KEYRING_USER: &str = "encryption-key";

#[derive(Clone)]
pub struct TokenCrypto {
    cipher: Aes256Gcm,
}

impl TokenCrypto {
    pub fn new() -> AppResult<Self> {
        let key = Self::get_or_create_key()?;
        let cipher =
            Aes256Gcm::new_from_slice(&key).map_err(|e| AppError::Crypto(e.to_string()))?;
        Ok(Self { cipher })
    }

    fn get_or_create_key() -> AppResult<[u8; KEY_SIZE]> {
        let entry = Entry::new(KEYRING_SERVICE, KEYRING_USER)
            .map_err(|e| AppError::Crypto(format!("Failed to access keyring: {}", e)))?;

        // Try to get existing key
        match entry.get_password() {
            Ok(hex_key) => {
                let key_bytes = hex::decode(&hex_key)
                    .map_err(|e| AppError::Crypto(format!("Invalid key format: {}", e)))?;

                if key_bytes.len() != KEY_SIZE {
                    return Err(AppError::Crypto(format!(
                        "Invalid key length: expected {}, got {}",
                        KEY_SIZE,
                        key_bytes.len()
                    )));
                }

                let mut key = [0u8; KEY_SIZE];
                key.copy_from_slice(&key_bytes);
                Ok(key)
            }
            Err(keyring::Error::NoEntry) => {
                // Generate new key
                let mut key = [0u8; KEY_SIZE];
                rand::rng().fill_bytes(&mut key);

                // Store in keyring
                let hex_key = hex::encode(key);
                entry
                    .set_password(&hex_key)
                    .map_err(|e| AppError::Crypto(format!("Failed to store key: {}", e)))?;

                Ok(key)
            }
            Err(e) => Err(AppError::Crypto(format!("Keyring error: {}", e))),
        }
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

    fn setup_mock_keyring() {
        // Use mock credential builder for testing to avoid depending on OS keychain
        keyring::set_default_credential_builder(keyring::mock::default_credential_builder());
    }

    #[test]
    fn test_encrypt_decrypt() {
        setup_mock_keyring();

        let crypto = TokenCrypto::new().unwrap();
        let plaintext = "test-token-12345";

        let encrypted = crypto.encrypt(plaintext).unwrap();
        let decrypted = crypto.decrypt(&encrypted).unwrap();

        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_encrypt_decrypt_multiple_values() {
        setup_mock_keyring();

        let crypto = TokenCrypto::new().unwrap();
        let tokens = ["token-1", "token-2", "longer-token-with-special-chars!@#$%"];

        for token in tokens {
            let encrypted = crypto.encrypt(token).unwrap();
            let decrypted = crypto.decrypt(&encrypted).unwrap();
            assert_eq!(token, decrypted);
        }
    }

    #[test]
    fn test_decrypt_invalid_data() {
        setup_mock_keyring();

        let crypto = TokenCrypto::new().unwrap();

        // Data too short (less than NONCE_SIZE)
        let short_data = vec![0u8; 5];
        assert!(crypto.decrypt(&short_data).is_err());

        // Invalid ciphertext (correct length but wrong content)
        let invalid_data = vec![0u8; 50];
        assert!(crypto.decrypt(&invalid_data).is_err());
    }
}
