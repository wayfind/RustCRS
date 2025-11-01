use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use once_cell::sync::Lazy;
use rand::Rng;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::utils::{AppError, Result};

// AES-256-CBC cipher types
type Aes256CbcEnc = cbc::Encryptor<aes::Aes256>;
type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;

/// Maximum buffer size for encryption/decryption (10 MB)
const MAX_BUFFER_SIZE: usize = 10 * 1024 * 1024;

/// Encryption constants matching Node.js implementation
const ENCRYPTION_SALT: &[u8] = b"salt";
const SCRYPT_LOG_N: u8 = 15; // N = 2^15 = 32768
const SCRYPT_R: u32 = 8;
const SCRYPT_P: u32 = 1;
const KEY_LENGTH: usize = 32; // 256 bits
const IV_LENGTH: usize = 16; // 128 bits

/// LRU cache entry
struct CacheEntry {
    value: String,
    expires_at: Instant,
}

/// LRU decrypt cache with TTL support
pub struct DecryptCache {
    cache: HashMap<String, CacheEntry>,
    max_size: usize,
    ttl: Duration,
    hits: usize,
    misses: usize,
}

impl DecryptCache {
    /// Create a new decrypt cache
    pub fn new(max_size: usize, ttl: Duration) -> Self {
        Self {
            cache: HashMap::with_capacity(max_size),
            max_size,
            ttl,
            hits: 0,
            misses: 0,
        }
    }

    /// Get a value from cache
    pub fn get(&mut self, key: &str) -> Option<String> {
        // Clean up expired entries first
        self.cleanup_expired();

        if let Some(entry) = self.cache.get(key) {
            if entry.expires_at > Instant::now() {
                self.hits += 1;
                return Some(entry.value.clone());
            } else {
                // Expired entry, remove it
                self.cache.remove(key);
            }
        }

        self.misses += 1;
        None
    }

    /// Set a value in cache
    pub fn set(&mut self, key: String, value: String) {
        // If cache is full, remove oldest entries
        if self.cache.len() >= self.max_size {
            // Simple LRU: remove 20% of entries
            let remove_count = self.max_size / 5;
            let keys_to_remove: Vec<String> = self
                .cache
                .iter()
                .take(remove_count)
                .map(|(k, _)| k.clone())
                .collect();

            for k in keys_to_remove {
                self.cache.remove(&k);
            }
        }

        let entry = CacheEntry {
            value,
            expires_at: Instant::now() + self.ttl,
        };

        self.cache.insert(key, entry);
    }

    /// Clean up expired entries
    fn cleanup_expired(&mut self) {
        let now = Instant::now();
        self.cache.retain(|_, entry| entry.expires_at > now);
    }

    /// Get cache statistics
    pub fn stats(&self) -> (usize, usize, usize, f64) {
        let total = self.hits + self.misses;
        let hit_rate = if total > 0 {
            (self.hits as f64 / total as f64) * 100.0
        } else {
            0.0
        };
        (self.cache.len(), self.hits, self.misses, hit_rate)
    }

    /// Clear all cache entries
    pub fn clear(&mut self) {
        self.cache.clear();
        self.hits = 0;
        self.misses = 0;
    }
}

/// Global decrypt cache (500 entries, 5 minutes TTL)
static DECRYPT_CACHE: Lazy<Mutex<DecryptCache>> =
    Lazy::new(|| Mutex::new(DecryptCache::new(500, Duration::from_secs(300))));

/// Cached encryption key (derived once for performance)
static ENCRYPTION_KEY_CACHE: Lazy<Mutex<Option<Vec<u8>>>> = Lazy::new(|| Mutex::new(None));

/// Crypto service for encryption/decryption operations
pub struct CryptoService {
    encryption_key_source: String,
}

impl CryptoService {
    /// Create a new crypto service
    pub fn new(encryption_key: String) -> Self {
        Self {
            encryption_key_source: encryption_key,
        }
    }

    /// Derive encryption key using Scrypt (with caching for performance)
    fn derive_key(&self) -> Result<Vec<u8>> {
        // Check cache first
        let mut cache = ENCRYPTION_KEY_CACHE
            .lock()
            .map_err(|e| AppError::InternalError(format!("Failed to lock key cache: {}", e)))?;

        if let Some(ref key) = *cache {
            return Ok(key.clone());
        }

        // Derive key using Scrypt
        let params = scrypt::Params::new(SCRYPT_LOG_N, SCRYPT_R, SCRYPT_P, KEY_LENGTH)
            .map_err(|e| AppError::InternalError(format!("Invalid scrypt params: {}", e)))?;

        let mut key = vec![0u8; KEY_LENGTH];
        scrypt::scrypt(
            self.encryption_key_source.as_bytes(),
            ENCRYPTION_SALT,
            &params,
            &mut key,
        )
        .map_err(|e| AppError::InternalError(format!("Scrypt key derivation failed: {}", e)))?;

        // Cache the derived key
        *cache = Some(key.clone());

        tracing::info!("🔑 Encryption key derived and cached for performance optimization");

        Ok(key)
    }

    /// Encrypt sensitive data using AES-256-CBC
    ///
    /// Format: {iv_hex}:{encrypted_hex}
    pub fn encrypt(&self, data: &str) -> Result<String> {
        if data.is_empty() {
            return Ok(String::new());
        }

        // Check buffer size limit
        if data.len() > MAX_BUFFER_SIZE {
            return Err(AppError::InternalError(format!(
                "Data too large for encryption: {} bytes (max: {} bytes)",
                data.len(),
                MAX_BUFFER_SIZE
            )));
        }

        // Derive encryption key
        let key = self.derive_key()?;

        // Generate random IV
        let mut iv = [0u8; IV_LENGTH];
        rand::thread_rng().fill(&mut iv);

        // Prepare buffer for encryption (data + padding)
        let data_bytes = data.as_bytes();
        let block_size = 16; // AES block size
        let padded_len = ((data_bytes.len() / block_size) + 1) * block_size;
        let mut buffer = vec![0u8; padded_len];
        buffer[..data_bytes.len()].copy_from_slice(data_bytes);

        // Encrypt data
        let cipher = Aes256CbcEnc::new(key.as_slice().into(), &iv.into());
        let encrypted = cipher
            .encrypt_padded_mut::<Pkcs7>(&mut buffer, data_bytes.len())
            .map_err(|e| AppError::InternalError(format!("Encryption failed: {:?}", e)))?;

        // Format: iv:encrypted (both in hex)
        let result = format!("{}:{}", hex::encode(iv), hex::encode(encrypted));

        Ok(result)
    }

    /// Decrypt sensitive data using AES-256-CBC
    ///
    /// Supports both:
    /// - New format: {iv_hex}:{encrypted_hex}
    /// - Old format: {encrypted_hex} (for backward compatibility)
    pub fn decrypt(&self, encrypted_data: &str) -> Result<String> {
        if encrypted_data.is_empty() {
            return Ok(String::new());
        }

        // Generate cache key (SHA256 hash of encrypted data)
        let cache_key = {
            let mut hasher = Sha256::new();
            hasher.update(encrypted_data.as_bytes());
            hex::encode(hasher.finalize())
        };

        // Check cache first
        {
            let mut cache = DECRYPT_CACHE.lock().map_err(|e| {
                AppError::InternalError(format!("Failed to lock decrypt cache: {}", e))
            })?;

            if let Some(cached) = cache.get(&cache_key) {
                return Ok(cached);
            }
        }

        // Decrypt data
        let decrypted = self.decrypt_internal(encrypted_data)?;

        // Cache the result
        {
            let mut cache = DECRYPT_CACHE.lock().map_err(|e| {
                AppError::InternalError(format!("Failed to lock decrypt cache: {}", e))
            })?;

            cache.set(cache_key, decrypted.clone());

            // Log cache stats periodically (every 1000 operations)
            let (size, hits, misses, hit_rate) = cache.stats();
            if (hits + misses) % 1000 == 0 && (hits + misses) > 0 {
                tracing::info!(
                    "📊 Decrypt cache stats: size={}, hits={}, misses={}, hit_rate={:.2}%",
                    size,
                    hits,
                    misses,
                    hit_rate
                );
            }
        }

        Ok(decrypted)
    }

    /// Internal decrypt implementation
    fn decrypt_internal(&self, encrypted_data: &str) -> Result<String> {
        // Check if new format (contains ':')
        if encrypted_data.contains(':') {
            let parts: Vec<&str> = encrypted_data.split(':').collect();
            if parts.len() == 2 {
                return self.decrypt_new_format(parts[0], parts[1]);
            }
        }

        // Old format fallback (for backward compatibility)
        // Note: This requires the encryption key to be compatible with Node.js's createDecipher
        // In practice, this may not work due to differences in key derivation
        // For now, we'll return an error suggesting re-encryption
        Err(AppError::InternalError(
            "Old encryption format detected. Please re-encrypt with new format.".to_string(),
        ))
    }

    /// Decrypt new format: {iv_hex}:{encrypted_hex}
    fn decrypt_new_format(&self, iv_hex: &str, encrypted_hex: &str) -> Result<String> {
        // Derive encryption key
        let key = self.derive_key()?;

        // Decode IV and encrypted data from hex
        let iv = hex::decode(iv_hex)
            .map_err(|e| AppError::InternalError(format!("Invalid IV hex: {}", e)))?;

        let mut encrypted = hex::decode(encrypted_hex)
            .map_err(|e| AppError::InternalError(format!("Invalid encrypted hex: {}", e)))?;

        if iv.len() != IV_LENGTH {
            return Err(AppError::InternalError(format!(
                "Invalid IV length: expected {}, got {}",
                IV_LENGTH,
                iv.len()
            )));
        }

        // Check buffer size limit
        if encrypted.len() > MAX_BUFFER_SIZE {
            return Err(AppError::InternalError(format!(
                "Encrypted data too large: {} bytes (max: {} bytes)",
                encrypted.len(),
                MAX_BUFFER_SIZE
            )));
        }

        // Decrypt data
        let cipher = Aes256CbcDec::new(key.as_slice().into(), iv.as_slice().into());
        let decrypted = cipher
            .decrypt_padded_mut::<Pkcs7>(&mut encrypted)
            .map_err(|e| AppError::InternalError(format!("Decryption failed: {:?}", e)))?;

        // Convert to UTF-8 string
        let result = String::from_utf8(decrypted.to_vec()).map_err(|e| {
            AppError::InternalError(format!("Invalid UTF-8 in decrypted data: {}", e))
        })?;

        Ok(result)
    }

    /// Get decrypt cache statistics
    pub fn cache_stats() -> Result<(usize, usize, usize, f64)> {
        let cache = DECRYPT_CACHE
            .lock()
            .map_err(|e| AppError::InternalError(format!("Failed to lock decrypt cache: {}", e)))?;

        Ok(cache.stats())
    }

    /// Clear decrypt cache
    pub fn clear_cache() -> Result<()> {
        let mut cache = DECRYPT_CACHE
            .lock()
            .map_err(|e| AppError::InternalError(format!("Failed to lock decrypt cache: {}", e)))?;

        cache.clear();
        tracing::info!("🧹 Decrypt cache cleared");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let crypto = CryptoService::new("test-encryption-key-32-chars!!".to_string());

        let original = "sensitive data 123";
        let encrypted = crypto.encrypt(original).expect("Encryption failed");

        // Encrypted data should be in format: {iv_hex}:{encrypted_hex}
        assert!(encrypted.contains(':'));
        assert!(!encrypted.is_empty());
        assert_ne!(encrypted, original);

        let decrypted = crypto.decrypt(&encrypted).expect("Decryption failed");
        assert_eq!(decrypted, original);
    }

    #[test]
    fn test_encrypt_empty_string() {
        let crypto = CryptoService::new("test-encryption-key-32-chars!!".to_string());

        let encrypted = crypto.encrypt("").expect("Encryption failed");
        assert_eq!(encrypted, "");

        let decrypted = crypto.decrypt("").expect("Decryption failed");
        assert_eq!(decrypted, "");
    }

    #[test]
    #[serial]
    fn test_decrypt_cache() {
        let crypto = CryptoService::new("test-encryption-key-cache-test-v2".to_string());

        let original = "unique cached data for test v2";
        let encrypted = crypto.encrypt(original).expect("Encryption failed");

        // Clear cache first
        CryptoService::clear_cache().expect("Failed to clear cache");

        // Get initial stats
        let (_, hits_before, misses_before, _) =
            CryptoService::cache_stats().expect("Failed to get stats");

        // First decrypt (cache miss)
        let decrypted1 = crypto.decrypt(&encrypted).expect("Decryption failed");
        assert_eq!(decrypted1, original);

        // Second decrypt (cache hit)
        let decrypted2 = crypto.decrypt(&encrypted).expect("Decryption failed");
        assert_eq!(decrypted2, original);

        // Third decrypt (cache hit)
        let decrypted3 = crypto.decrypt(&encrypted).expect("Decryption failed");
        assert_eq!(decrypted3, original);

        // Check cache stats
        let (size_after, hits_after, misses_after, hit_rate) =
            CryptoService::cache_stats().expect("Failed to get stats");

        // Should have at least one entry in cache
        assert!(size_after >= 1, "Cache should have at least one entry");

        // Should have at least one miss (from first decrypt)
        let miss_delta = misses_after - misses_before;
        assert!(
            miss_delta >= 1,
            "Should have at least one cache miss, got {}",
            miss_delta
        );

        // Should have at least two hits (from second and third decrypt)
        let hit_delta = hits_after - hits_before;
        assert!(
            hit_delta >= 2,
            "Should have at least two cache hits, got {}",
            hit_delta
        );

        // Hit rate should be > 0
        assert!(hit_rate > 0.0, "Hit rate should be positive");
    }

    #[test]
    fn test_encrypt_different_iv() {
        let crypto = CryptoService::new("test-encryption-key-32-chars!!".to_string());

        let original = "same data";
        let encrypted1 = crypto.encrypt(original).expect("Encryption failed");
        let encrypted2 = crypto.encrypt(original).expect("Encryption failed");

        // Different encryptions should have different IVs
        assert_ne!(encrypted1, encrypted2);

        // But both should decrypt to the same value
        let decrypted1 = crypto.decrypt(&encrypted1).expect("Decryption failed");
        let decrypted2 = crypto.decrypt(&encrypted2).expect("Decryption failed");
        assert_eq!(decrypted1, original);
        assert_eq!(decrypted2, original);
    }

    #[test]
    #[serial]
    fn test_key_caching() {
        let crypto1 = CryptoService::new("test-encryption-key-32-chars!!".to_string());
        let crypto2 = CryptoService::new("test-encryption-key-32-chars!!".to_string());

        // Both should use the same cached key
        let key1 = crypto1.derive_key().expect("Key derivation failed");
        let key2 = crypto2.derive_key().expect("Key derivation failed");

        assert_eq!(key1, key2);
    }

    #[test]
    fn test_invalid_encrypted_data() {
        let crypto = CryptoService::new("test-encryption-key-32-chars!!".to_string());

        // Invalid hex
        let result = crypto.decrypt("invalid:hex");
        assert!(result.is_err());

        // Invalid format
        let result = crypto.decrypt("no_colon_format");
        assert!(result.is_err());
    }
}
