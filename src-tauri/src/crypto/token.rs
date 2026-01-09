use aes_gcm::{
    aead::{rand_core::RngCore, Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use thiserror::Error;

const NONCE_SIZE: usize = 12;
const KEY_SIZE: usize = 32;
const KEYRING_SERVICE: &str = "local-code-agent";
const KEYRING_USER: &str = "encryption-key";

#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("Encryption failed")]
    EncryptionFailed,
    #[error("Decryption failed")]
    DecryptionFailed,
    #[error("Invalid data format")]
    InvalidFormat,
    #[error("Keychain error: {0}")]
    KeychainError(String),
}

pub struct TokenCrypto {
    cipher: Aes256Gcm,
}

impl TokenCrypto {
    /// Create TokenCrypto with key from keychain or generate new one
    pub fn new() -> Result<Self, CryptoError> {
        let key = Self::get_or_generate_key()?;
        let cipher = Aes256Gcm::new_from_slice(&key).map_err(|_| CryptoError::EncryptionFailed)?;
        Ok(Self { cipher })
    }

    /// Get key from keychain or generate and store new one
    /// Falls back to file-based storage if keychain is unavailable
    fn get_or_generate_key() -> Result<[u8; KEY_SIZE], CryptoError> {
        // Try keychain first
        match keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER) {
            Ok(entry) => {
                match entry.get_password() {
                    Ok(key_hex) => {
                        // Decode existing key from hex
                        let key = hex::decode(&key_hex).map_err(|_| CryptoError::InvalidFormat)?;
                        if key.len() != KEY_SIZE {
                            return Err(CryptoError::InvalidFormat);
                        }
                        let mut arr = [0u8; KEY_SIZE];
                        arr.copy_from_slice(&key);
                        return Ok(arr);
                    }
                    Err(_) => {
                        // Generate and store new key
                        let mut key = [0u8; KEY_SIZE];
                        OsRng.fill_bytes(&mut key);
                        let key_hex = hex::encode(key);
                        if entry.set_password(&key_hex).is_ok() {
                            tracing::info!("Stored new encryption key in keychain");
                            return Ok(key);
                        }
                        // Fall through to file-based storage
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Keychain not available: {:?}", e);
                // Fall through to file-based storage
            }
        }

        // Fallback: file-based key storage (less secure)
        tracing::warn!(
            "Keychain unavailable, falling back to file-based key storage. \
             This is less secure than keychain storage."
        );
        Self::get_or_generate_key_from_file()
    }

    /// Set restrictive file permissions on Windows using ACL
    #[cfg(windows)]
    fn set_windows_file_permissions(path: &std::path::Path) -> Result<(), CryptoError> {
        use std::process::Command;

        // Use icacls to restrict file access to current user only
        // First, disable inheritance and remove all existing permissions
        let output = Command::new("icacls")
            .args([
                path.to_str().unwrap_or_default(),
                "/inheritance:r",
                "/grant:r",
                &format!(
                    "{}:F",
                    std::env::var("USERNAME").unwrap_or_else(|_| "SYSTEM".to_string())
                ),
            ])
            .output();

        match output {
            Ok(result) if result.status.success() => {
                tracing::debug!("Set Windows ACL permissions on key file");
                Ok(())
            }
            Ok(result) => {
                tracing::warn!(
                    "Failed to set Windows ACL: {}",
                    String::from_utf8_lossy(&result.stderr)
                );
                // Don't fail - the file is still created, just with default permissions
                Ok(())
            }
            Err(e) => {
                tracing::warn!("Failed to execute icacls: {}", e);
                Ok(())
            }
        }
    }

    /// Fallback: store encryption key in application data directory
    fn get_or_generate_key_from_file() -> Result<[u8; KEY_SIZE], CryptoError> {
        let key_path = directories::ProjectDirs::from("com", "local-code-agent", "LocalCodeAgent")
            .ok_or_else(|| CryptoError::KeychainError("Cannot determine data directory".into()))?
            .data_local_dir()
            .join(".encryption_key");

        if key_path.exists() {
            let key_hex =
                std::fs::read_to_string(&key_path).map_err(|_| CryptoError::EncryptionFailed)?;
            let key = hex::decode(key_hex.trim()).map_err(|_| CryptoError::InvalidFormat)?;
            if key.len() != KEY_SIZE {
                return Err(CryptoError::InvalidFormat);
            }
            let mut arr = [0u8; KEY_SIZE];
            arr.copy_from_slice(&key);
            Ok(arr)
        } else {
            // Generate and store new key
            if let Some(parent) = key_path.parent() {
                std::fs::create_dir_all(parent).map_err(|_| CryptoError::EncryptionFailed)?;
            }
            let mut key = [0u8; KEY_SIZE];
            OsRng.fill_bytes(&mut key);
            let key_hex = hex::encode(key);
            std::fs::write(&key_path, &key_hex).map_err(|_| CryptoError::EncryptionFailed)?;

            // Set restrictive permissions
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600))
                    .map_err(|_| CryptoError::EncryptionFailed)?;
            }

            #[cfg(windows)]
            {
                Self::set_windows_file_permissions(&key_path)?;
            }

            tracing::info!("Stored new encryption key in file: {:?}", key_path);
            Ok(key)
        }
    }

    /// Encrypt plaintext and return nonce + ciphertext
    pub fn encrypt(&self, plaintext: &str) -> Result<Vec<u8>, CryptoError> {
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = self
            .cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|_| CryptoError::EncryptionFailed)?;

        // Prepend nonce to ciphertext
        let mut result = nonce_bytes.to_vec();
        result.extend(ciphertext);
        Ok(result)
    }

    /// Decrypt ciphertext (with prepended nonce)
    pub fn decrypt(&self, encrypted: &[u8]) -> Result<String, CryptoError> {
        if encrypted.len() < NONCE_SIZE {
            return Err(CryptoError::InvalidFormat);
        }

        let (nonce_bytes, ciphertext) = encrypted.split_at(NONCE_SIZE);
        let nonce = Nonce::from_slice(nonce_bytes);

        let plaintext = self
            .cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| CryptoError::DecryptionFailed)?;

        String::from_utf8(plaintext).map_err(|_| CryptoError::DecryptionFailed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        // Use file-based key for testing
        let crypto = TokenCrypto::new().unwrap();
        let plaintext = "ghp_secret_token_12345";

        let encrypted = crypto.encrypt(plaintext).unwrap();
        assert!(encrypted.len() > NONCE_SIZE);

        let decrypted = crypto.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_different_nonces_produce_different_ciphertexts() {
        let crypto = TokenCrypto::new().unwrap();
        let plaintext = "same_plaintext";

        let encrypted1 = crypto.encrypt(plaintext).unwrap();
        let encrypted2 = crypto.encrypt(plaintext).unwrap();

        // Nonces (first 12 bytes) should be different
        assert_ne!(&encrypted1[..NONCE_SIZE], &encrypted2[..NONCE_SIZE]);
        // Both should decrypt to same plaintext
        assert_eq!(crypto.decrypt(&encrypted1).unwrap(), plaintext);
        assert_eq!(crypto.decrypt(&encrypted2).unwrap(), plaintext);
    }

    #[test]
    fn test_decrypt_invalid_data() {
        let crypto = TokenCrypto::new().unwrap();

        // Too short
        assert!(crypto.decrypt(&[0u8; 5]).is_err());

        // Invalid ciphertext
        assert!(crypto.decrypt(&[0u8; 32]).is_err());
    }
}
