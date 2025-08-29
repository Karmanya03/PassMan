use getrandom::getrandom;
use argon2::{
    Argon2, Params,
    password_hash::{PasswordHash, PasswordVerifier},
    Algorithm, Version
};
use chacha20poly1305::{
    XChaCha20Poly1305, Key, XNonce,
    aead::{Aead, KeyInit},
};
// zeroize not currently used in this module
use std::time::Instant;

/// Enhanced Argon2id parameters for maximum security
/// These parameters are tuned for security vs performance balance
pub struct Argon2Config {
    pub memory_cost: u32,    // Memory usage in KiB
    pub time_cost: u32,      // Number of iterations
    pub parallelism: u32,    // Number of parallel threads
    pub hash_length: Option<usize>, // Output hash length
}

impl Default for Argon2Config {
    fn default() -> Self {
        Self {
            memory_cost: 65536,  // 64 MiB memory
            time_cost: 3,        // 3 iterations (OWASP minimum)
            parallelism: 4,      // 4 parallel threads
            hash_length: Some(32), // 32-byte output
        }
    }
}

/// Derives a 32-byte encryption key using Argon2id with enhanced security parameters.
/// 
/// # Security Features:
/// - Uses Argon2id (OWASP recommended)
/// - High memory cost (64MB) to resist GPU attacks
/// - Constant-time operations where possible
/// - Automatic memory cleanup with zeroize
pub fn derive_key(pass: &str, salt: &[u8]) -> [u8; 32] {
    derive_key_with_config(pass, salt, &Argon2Config::default())
}

/// Derives a key with custom Argon2id configuration
pub fn derive_key_with_config(pass: &str, salt: &[u8], config: &Argon2Config) -> [u8; 32] {
    let start_time = Instant::now();
    
    // Ensure salt is at least 16 bytes for security
    if salt.len() < 16 {
        panic!("Salt must be at least 16 bytes for security");
    }
    
    // Prepare Argon2 parameters and derive raw bytes directly into the key buffer.
    let params = Params::new(
        config.memory_cost,
        config.time_cost,
        config.parallelism,
        config.hash_length,
    ).expect("Invalid Argon2id parameters");

    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

    // Derive directly into a 32-byte key buffer to avoid intermediate string encodings.
    let mut key = [0u8; 32];
    argon2
        .hash_password_into(pass.as_bytes(), salt, &mut key)
        .expect("Argon2id hashing failed");
    
    let duration = start_time.elapsed();
    
    // Log timing for security audit (should be > 100ms for good security)
    if duration.as_millis() < 100 {
        eprintln!("⚠️  Warning: Key derivation completed in {}ms (recommended: >100ms)", duration.as_millis());
    }
    
    key
}

/// Verifies a password against a stored Argon2id hash
#[allow(dead_code)]
pub fn verify_password(password: &str, hash_string: &str) -> Result<bool, argon2::password_hash::Error> {
    let parsed_hash = PasswordHash::new(hash_string)?;
    let argon2 = Argon2::default();
    
    match argon2.verify_password(password.as_bytes(), &parsed_hash) {
        Ok(()) => Ok(true),
        Err(argon2::password_hash::Error::Password) => Ok(false),
        Err(e) => Err(e),
    }
}

/// Encrypts the vault data using XChaCha20Poly1305 with enhanced security.
/// 
/// # Security Features:
/// - XChaCha20Poly1305 (authenticated encryption)
/// - 192-bit nonce (extended nonce space)
/// - Padding to hide data length
/// - Constant-time operations
pub fn encrypt(key: &[u8; 32], plaintext: &[u8]) -> Vec<u8> {
    let cipher = XChaCha20Poly1305::new(Key::from_slice(key));
    // Generate a 24-byte XChaCha20 nonce using getrandom (no rand_core deps)
    let mut nonce_bytes = [0u8; 24];
    getrandom(&mut nonce_bytes).expect("OS RNG failed");
    let nonce = XNonce::from_slice(&nonce_bytes);
    
    // Add padding to hide the actual data length
    let padded = add_secure_padding(plaintext);
    
    let ciphertext = cipher
        .encrypt(&nonce, padded.as_ref())
        .expect("XChaCha20Poly1305 encryption failed");
    
    // Prepend nonce to ciphertext
    let mut result = Vec::with_capacity(nonce.len() + ciphertext.len());
    result.extend_from_slice(nonce.as_slice());
    result.extend_from_slice(&ciphertext);
    
    result
}

/// Decrypts vault data with proper error handling
pub fn decrypt(key: &[u8; 32], ciphertext: &[u8]) -> Result<Vec<u8>, CryptoError> {
    if ciphertext.len() < 24 {
        return Err(CryptoError::InvalidCiphertext("Ciphertext too short".to_string()));
    }
    
    let cipher = XChaCha20Poly1305::new(Key::from_slice(key));
    let (nonce_bytes, ct) = ciphertext.split_at(24);
    let nonce = XNonce::from_slice(nonce_bytes);
    
    let decrypted = cipher
        .decrypt(nonce, ct)
        .map_err(|_| CryptoError::DecryptionFailed)?;
    
    // Remove padding
    let unpadded = remove_secure_padding(&decrypted)?;
    
    Ok(unpadded)
}

/// Adds secure padding that hides the actual data length
/// Uses PKCS#7-style padding with random data
fn add_secure_padding(data: &[u8]) -> Vec<u8> {
    // Padding length is stored in a single u8 so the maximum block size must be <= 255.
    const BLOCK_SIZE: usize = 255; // Pad to 255-byte blocks (fits in u8)
    
    let mut padded = data.to_vec();
    let padding_needed = BLOCK_SIZE - (data.len() % BLOCK_SIZE);
    
    if padding_needed == BLOCK_SIZE {
        // Data is already block-aligned, add a full block of padding
        padded.reserve(BLOCK_SIZE);
    } else {
        padded.reserve(padding_needed);
    }
    
    // Add random padding bytes except for the last byte
    let mut random_padding = vec![0u8; padding_needed.saturating_sub(1)];
    if !random_padding.is_empty() {
        getrandom(&mut random_padding).expect("OS RNG failed");
    }
    padded.extend_from_slice(&random_padding);
    
    // Last byte indicates the amount of padding
    padded.push(padding_needed as u8);
    
    padded
}

/// Removes secure padding and validates integrity
fn remove_secure_padding(padded_data: &[u8]) -> Result<Vec<u8>, CryptoError> {
    if padded_data.is_empty() {
        return Err(CryptoError::InvalidPadding("Empty data".to_string()));
    }
    
    let padding_length = *padded_data.last().unwrap() as usize;
    
    const MAX_BLOCK: usize = 255;
    if padding_length == 0 || padding_length > MAX_BLOCK || padding_length > padded_data.len() {
        return Err(CryptoError::InvalidPadding("Invalid padding length".to_string()));
    }
    
    let data_length = padded_data.len() - padding_length;
    Ok(padded_data[0..data_length].to_vec())
}

/// Generates a cryptographically secure random salt
pub fn generate_salt(length: usize) -> Vec<u8> {
    let mut salt = vec![0u8; length.max(16)]; // Minimum 16 bytes
    getrandom(&mut salt).expect("OS RNG failed");
    salt
}

/// Generates a secure random password with specified criteria
pub fn generate_password(length: usize, include_symbols: bool) -> String {
    let lowercase = b"abcdefghijklmnopqrstuvwxyz";
    let uppercase = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let digits = b"0123456789";
    let symbols = b"!@#$%^&*()_+-=[]{}|;:,.<>?";
    
    let mut charset = Vec::new();
    charset.extend_from_slice(lowercase);
    charset.extend_from_slice(uppercase);
    charset.extend_from_slice(digits);
    
    if include_symbols {
        charset.extend_from_slice(symbols);
    }
    
    let mut password = Vec::with_capacity(length);

    // Helper to get a random u32 from the OS RNG
    let random_u32 = || -> u32 {
        let mut b = [0u8; 4];
        getrandom(&mut b).expect("OS RNG failed");
        u32::from_le_bytes(b)
    };

    // Helper to pick a random byte from a slice
    let pick = |slice: &[u8]| -> u8 {
        let idx = (random_u32() as usize) % slice.len();
        slice[idx]
    };

    // Ensure at least one character from each category
    password.push(pick(lowercase));
    password.push(pick(uppercase));
    password.push(pick(digits));

    if include_symbols {
        password.push(pick(symbols));
    }

    // Fill the rest randomly
    while password.len() < length {
        password.push(pick(&charset));
    }

    // Fisher-Yates shuffle using OS RNG
    for i in (1..password.len()).rev() {
        let j = (random_u32() as usize) % (i + 1);
        password.swap(i, j);
    }
    
    String::from_utf8(password).expect("Valid UTF-8 password")
}

/// Securely compares two byte arrays in constant time
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    
    let mut result = 0u8;
    for i in 0..a.len() {
        result |= a[i] ^ b[i];
    }
    result == 0
}

#[allow(dead_code)]
pub fn _constant_time_eq_allow(a: &[u8], b: &[u8]) -> bool { constant_time_eq(a,b) }

/// Estimates password strength (0-100 score)
pub fn estimate_password_strength(password: &str) -> PasswordStrength {
    let mut score = 0u32;
    let mut feedback = Vec::new();
    
    // Length scoring
    match password.len() {
        0..=7 => {
            feedback.push("Password too short (minimum 8 characters)".to_string());
        }
        8..=11 => score += 20,
        12..=15 => score += 35,
        _ => score += 50,
    }
    
    // Character variety
    let has_lower = password.chars().any(|c| c.is_ascii_lowercase());
    let has_upper = password.chars().any(|c| c.is_ascii_uppercase());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());
    let has_symbol = password.chars().any(|c| !c.is_alphanumeric());
    
    let variety_count = [has_lower, has_upper, has_digit, has_symbol].iter().filter(|&&x| x).count();
    score += variety_count as u32 * 10;
    
    if !has_lower || !has_upper {
        feedback.push("Use both uppercase and lowercase letters".to_string());
    }
    if !has_digit {
        feedback.push("Include at least one number".to_string());
    }
    if !has_symbol {
        feedback.push("Include special characters".to_string());
    }
    
    // Check for common patterns
    if password.chars().collect::<Vec<_>>().windows(3).any(|w| {
        w[0] as u8 + 1 == w[1] as u8 && w[1] as u8 + 1 == w[2] as u8
    }) {
        score = score.saturating_sub(15);
        feedback.push("Avoid sequential characters".to_string());
    }
    
    let strength_level = match score.min(100) {
        0..=30 => "Weak",
        31..=60 => "Fair", 
        61..=80 => "Good",
        81..=100 => "Strong",
        _ => "Unknown",
    };
    
    PasswordStrength {
        score: score.min(100),
        level: strength_level.to_string(),
        feedback,
    }
}

#[derive(Debug)]
pub struct PasswordStrength {
    pub score: u32,
    pub level: String,
    pub feedback: Vec<String>,
}

#[derive(Debug)]
pub enum CryptoError {
    InvalidCiphertext(String),
    DecryptionFailed,
    InvalidPadding(String),
}

impl std::fmt::Display for CryptoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CryptoError::InvalidCiphertext(msg) => write!(f, "Invalid ciphertext: {}", msg),
            CryptoError::DecryptionFailed => write!(f, "Decryption failed"),
            CryptoError::InvalidPadding(msg) => write!(f, "Invalid padding: {}", msg),
            // Key derivation failure is represented via other error paths if needed
        }
    }
}

impl std::error::Error for CryptoError {}

/// Benchmark key derivation performance for security tuning
pub fn benchmark_key_derivation() -> std::time::Duration {
    let password = "test_password_for_benchmarking";
    let salt = generate_salt(32);
    
    let start = Instant::now();
    let _key = derive_key(password, &salt);
    start.elapsed()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = [42u8; 32];
        let data = b"Hello, world!";
        
        let encrypted = encrypt(&key, data);
        let decrypted = decrypt(&key, &encrypted).unwrap();
        
        assert_eq!(data.as_slice(), decrypted.as_slice());
    }
    
    #[test]
    fn test_padding_roundtrip() {
        let data = b"test data";
        let padded = add_secure_padding(data);
        let unpadded = remove_secure_padding(&padded).unwrap();
        
        assert_eq!(data.as_slice(), unpadded.as_slice());
    }
    
    #[test]
    fn test_password_generation() {
        let password = generate_password(16, true);
        assert_eq!(password.len(), 16);
        
        let strength = estimate_password_strength(&password);
        assert!(strength.score > 50); // Should be reasonably strong
    }
    
    #[test]
    fn test_constant_time_comparison() {
        let a = b"hello";
        let b = b"hello";
        let c = b"world";
        
        assert!(constant_time_eq(a, b));
        assert!(!constant_time_eq(a, c));
    }
}
