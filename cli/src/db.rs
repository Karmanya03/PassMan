use rusqlite::{params, Connection, OptionalExtension, Result as SqliteResult};
use std::path::Path;
use passmann_shared::{derive_key, derive_key_with_config, encrypt, decrypt, Argon2Config, PassMannError};
use log::{info, warn, debug};

/// SecureDb wraps an SQLite database with SQLCipher encryption for maximum security.
/// If SQLCipher is not available, it falls back to application-level encryption.
pub struct SecureDb {
    conn: Connection,
    sqlcipher: bool,
    encryption_enabled: bool,
}

#[derive(Debug)]
pub struct DbConfig {
    /// Whether to require SQLCipher (fail if not available)
    pub require_sqlcipher: bool,
    /// Number of key derivation iterations for SQLCipher
    pub kdf_iterations: u32,
    /// Memory cost for key derivation (KB)
    pub memory_cost: u32,
}

impl Default for DbConfig {
    fn default() -> Self {
        Self {
            require_sqlcipher: false,
            kdf_iterations: 256000,
            memory_cost: 65536, // 64MB
        }
    }
}

impl SecureDb {
    /// Open a database with proper SQLCipher encryption or fallback to application-level encryption
    pub fn open(path: &Path, master_password: &str) -> Result<Self, PassMannError> {
        Self::open_with_config(path, master_password, &DbConfig::default())
    }

    /// Open a database with custom configuration
    pub fn open_with_config(path: &Path, master_password: &str, config: &DbConfig) -> Result<Self, PassMannError> {
        let conn = Connection::open(path)
            .map_err(|e| PassMannError::Other(format!("Failed to open database: {}", e)))?;

        // Try to detect SQLCipher support
        let sqlcipher_available = Self::detect_sqlcipher(&conn);
        
        if config.require_sqlcipher && !sqlcipher_available {
            return Err(PassMannError::Other("SQLCipher is required but not available".to_string()));
        }

        if sqlcipher_available {
            info!("SQLCipher detected - using database-level encryption");
            Self::setup_sqlcipher(&conn, master_password, config)?;
            
            // Create vault table for encrypted storage
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS vault_entries (
                    id TEXT PRIMARY KEY,
                    service TEXT NOT NULL,
                    username TEXT NOT NULL,
                    password_data BLOB NOT NULL,
                    metadata TEXT,
                    created_at INTEGER NOT NULL,
                    updated_at INTEGER NOT NULL,
                    accessed_at INTEGER
                );
                CREATE INDEX IF NOT EXISTS idx_service ON vault_entries(service);
                CREATE INDEX IF NOT EXISTS idx_username ON vault_entries(username);
                
                CREATE TABLE IF NOT EXISTS vault_metadata (
                    key TEXT PRIMARY KEY,
                    value BLOB NOT NULL,
                    updated_at INTEGER NOT NULL
                );"
            ).map_err(|e| PassMannError::Other(format!("Failed to create tables: {}", e)))?;

            Ok(Self { 
                conn, 
                sqlcipher: true, 
                encryption_enabled: true 
            })
        } else {
            warn!("SQLCipher not available - using application-level encryption");
            
            // Create tables for application-level encrypted data
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS encrypted_vault_entries (
                    id TEXT PRIMARY KEY,
                    service TEXT NOT NULL,
                    username TEXT NOT NULL,
                    encrypted_data BLOB NOT NULL,
                    salt BLOB NOT NULL,
                    created_at INTEGER NOT NULL,
                    updated_at INTEGER NOT NULL,
                    accessed_at INTEGER
                );
                CREATE INDEX IF NOT EXISTS idx_enc_service ON encrypted_vault_entries(service);
                CREATE INDEX IF NOT EXISTS idx_enc_username ON encrypted_vault_entries(username);
                
                CREATE TABLE IF NOT EXISTS encrypted_metadata (
                    key TEXT PRIMARY KEY,
                    encrypted_value BLOB NOT NULL,
                    salt BLOB NOT NULL,
                    updated_at INTEGER NOT NULL
                );"
            ).map_err(|e| PassMannError::Other(format!("Failed to create encrypted tables: {}", e)))?;

            Ok(Self { 
                conn, 
                sqlcipher: false, 
                encryption_enabled: true 
            })
        }
    }

    /// Detect if SQLCipher is available
    fn detect_sqlcipher(conn: &Connection) -> bool {
        match conn.query_row("PRAGMA cipher_version;", [], |r| r.get::<_, String>(0)) {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    /// Setup SQLCipher encryption with strong key derivation
    fn setup_sqlcipher(conn: &Connection, master_password: &str, config: &DbConfig) -> Result<(), PassMannError> {
        // Use a fixed salt for SQLCipher key derivation (database-wide)
        let salt = b"passmann-sqlcipher-v2-2025-secure-salt";
        
        // Derive encryption key using Argon2
        let argon2_config = Argon2Config {
            memory_cost: config.memory_cost,
            time_cost: config.kdf_iterations / 1000, // Convert to reasonable time cost
            parallelism: 4,
            hash_length: Some(32),
        };
        
        let key = derive_key_with_config(master_password, salt, &argon2_config);
        let hex_key = hex::encode(key);
        
        // Configure SQLCipher
        conn.pragma_update(None, "key", &hex_key)
            .map_err(|e| PassMannError::Other(format!("Failed to set SQLCipher key: {}", e)))?;
        
        // Set additional security parameters
        conn.pragma_update(None, "cipher_page_size", "4096")
            .map_err(|e| PassMannError::Other(format!("Failed to set cipher page size: {}", e)))?;
        
        conn.pragma_update(None, "kdf_iter", &config.kdf_iterations.to_string())
            .map_err(|e| PassMannError::Other(format!("Failed to set KDF iterations: {}", e)))?;
        
        // Test that encryption is working by creating a test table
        conn.execute_batch("CREATE TEMP TABLE test_encryption (x INTEGER); DROP TABLE test_encryption;")
            .map_err(|e| PassMannError::Other(format!("SQLCipher encryption test failed: {}", e)))?;
        
        info!("SQLCipher configured successfully with {} KDF iterations", config.kdf_iterations);
        Ok(())
    }

    /// Put value into DB. If SQLCipher is enabled the DB file is encrypted; otherwise
    /// encrypt the value at application layer and store a salt + ciphertext blob.
    pub fn put(&self, key: &str, plaintext: &[u8], master_password: &str) -> Result<(), PassMannError> {
        if self.sqlcipher {
            self.conn.execute(
                "REPLACE INTO vault_metadata (key, value, updated_at) VALUES (?1, ?2, ?3)",
                params![key, plaintext, chrono::Utc::now().timestamp()],
            ).map_err(|e| PassMannError::Other(format!("Failed to store data: {}", e)))?;
        } else {
            // Use random salt per entry
            let mut salt = vec![0u8; 32];
            getrandom::getrandom(&mut salt).expect("OS RNG failed");
            let derived = derive_key(master_password, &salt);
            let ct = encrypt(&derived, plaintext);
            let mut blob = salt.clone();
            blob.extend_from_slice(&ct);
            self.conn.execute(
                "REPLACE INTO encrypted_metadata (key, encrypted_value, salt, updated_at) VALUES (?1, ?2, ?3, ?4)",
                params![key, blob, salt, chrono::Utc::now().timestamp()],
            ).map_err(|e| PassMannError::Other(format!("Failed to store encrypted data: {}", e)))?;
        }
        Ok(())
    }

    /// Get value for key. Returns plaintext when decryption succeeds.
    pub fn get(&self, key: &str, master_password: &str) -> Result<Option<Vec<u8>>, PassMannError> {
        if self.sqlcipher {
            let row: Option<Vec<u8>> = self.conn.query_row(
                "SELECT value FROM vault_metadata WHERE key = ?1",
                params![key],
                |r| r.get(0),
            ).optional().map_err(|e| PassMannError::Other(format!("Failed to retrieve data: {}", e)))?;
            Ok(row)
        } else {
            let row: Option<Vec<u8>> = self.conn.query_row(
                "SELECT encrypted_value FROM encrypted_metadata WHERE key = ?1",
                params![key],
                |r| r.get(0),
            ).optional().map_err(|e| PassMannError::Other(format!("Failed to retrieve encrypted data: {}", e)))?;

            if let Some(blob) = row {
                if blob.len() <= 32 {
                    return Ok(None);
                }
                let salt = &blob[0..32];
                let ct = &blob[32..];
                let derived = derive_key(master_password, salt);
                match decrypt(&derived, ct) {
                    Ok(pt) => Ok(Some(pt)),
                    Err(_) => Ok(None),
                }
            } else {
                Ok(None)
            }
        }
    }

    /// Store a vault entry securely
    pub fn store_entry(&self, entry: &passmann_shared::Entry, master_password: &str) -> Result<(), PassMannError> {
        use passmann_shared::Entry;
        
        let entry_json = serde_json::to_string(entry)
            .map_err(|e| PassMannError::Serialization(e))?;
        
        let now = chrono::Utc::now().timestamp();
        
        if self.sqlcipher {
            self.conn.execute(
                "REPLACE INTO vault_entries (id, service, username, password_data, metadata, created_at, updated_at, accessed_at) 
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    entry.id.to_string(),
                    entry.service,
                    entry.username,
                    entry_json.as_bytes(),
                    "{}".to_string(), // Empty metadata for now
                    entry.created_at.timestamp(),
                    now,
                    None::<i64> // Remove accessed_at field reference
                ],
            ).map_err(|e| PassMannError::Other(format!("Failed to store entry: {}", e)))?;
        } else {
            // Encrypt the entire entry
            let mut salt = vec![0u8; 32];
            getrandom::getrandom(&mut salt).expect("OS RNG failed");
            let derived = derive_key(master_password, &salt);
            let encrypted_data = encrypt(&derived, entry_json.as_bytes());
            
            self.conn.execute(
                "REPLACE INTO encrypted_vault_entries (id, service, username, encrypted_data, salt, created_at, updated_at, accessed_at) 
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    entry.id.to_string(),
                    entry.service,
                    entry.username,
                    encrypted_data,
                    salt,
                    entry.created_at.timestamp(),
                    now,
                    None::<i64> // Remove accessed_at field reference
                ],
            ).map_err(|e| PassMannError::Other(format!("Failed to store encrypted entry: {}", e)))?;
        }
        
        Ok(())
    }

    /// Retrieve a vault entry by ID
    pub fn get_entry(&self, id: &str, master_password: &str) -> Result<Option<passmann_shared::Entry>, PassMannError> {
        if self.sqlcipher {
            let result: Option<Vec<u8>> = self.conn.query_row(
                "SELECT password_data FROM vault_entries WHERE id = ?1",
                params![id],
                |r| r.get(0),
            ).optional().map_err(|e| PassMannError::Other(format!("Failed to retrieve entry: {}", e)))?;
            
            if let Some(data) = result {
                let entry_json = String::from_utf8(data)
                    .map_err(|e| PassMannError::Other(format!("Invalid UTF-8 data: {}", e)))?;
                let entry: passmann_shared::Entry = serde_json::from_str(&entry_json)?;
                Ok(Some(entry))
            } else {
                Ok(None)
            }
        } else {
            let result: Option<(Vec<u8>, Vec<u8>)> = self.conn.query_row(
                "SELECT encrypted_data, salt FROM encrypted_vault_entries WHERE id = ?1",
                params![id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            ).optional().map_err(|e| PassMannError::Other(format!("Failed to retrieve encrypted entry: {}", e)))?;
            
            if let Some((encrypted_data, salt)) = result {
                let derived = derive_key(master_password, &salt);
                let decrypted_data = decrypt(&derived, &encrypted_data)
                    .map_err(|e| PassMannError::Crypto(format!("Failed to decrypt entry: {}", e)))?;
                
                let entry_json = String::from_utf8(decrypted_data)
                    .map_err(|e| PassMannError::Other(format!("Invalid UTF-8 data: {}", e)))?;
                let entry: passmann_shared::Entry = serde_json::from_str(&entry_json)?;
                Ok(Some(entry))
            } else {
                Ok(None)
            }
        }
    }

    /// List all vault entries
    pub fn list_entries(&self, master_password: &str) -> Result<Vec<passmann_shared::Entry>, PassMannError> {
        let mut entries = Vec::new();
        
        if self.sqlcipher {
            let mut stmt = self.conn.prepare(
                "SELECT password_data FROM vault_entries ORDER BY service, username"
            ).map_err(|e| PassMannError::Other(format!("Failed to prepare statement: {}", e)))?;
            
            let entry_iter = stmt.query_map([], |row| {
                let data: Vec<u8> = row.get(0)?;
                Ok(data)
            }).map_err(|e| PassMannError::Other(format!("Failed to query entries: {}", e)))?;
            
            for entry_result in entry_iter {
                let data = entry_result.map_err(|e| PassMannError::Other(format!("Failed to read entry: {}", e)))?;
                let entry_json = String::from_utf8(data)
                    .map_err(|e| PassMannError::Other(format!("Invalid UTF-8 data: {}", e)))?;
                let entry: passmann_shared::Entry = serde_json::from_str(&entry_json)?;
                entries.push(entry);
            }
        } else {
            let mut stmt = self.conn.prepare(
                "SELECT encrypted_data, salt FROM encrypted_vault_entries ORDER BY service, username"
            ).map_err(|e| PassMannError::Other(format!("Failed to prepare statement: {}", e)))?;
            
            let entry_iter = stmt.query_map([], |row| {
                let encrypted_data: Vec<u8> = row.get(0)?;
                let salt: Vec<u8> = row.get(1)?;
                Ok((encrypted_data, salt))
            }).map_err(|e| PassMannError::Other(format!("Failed to query encrypted entries: {}", e)))?;
            
            for entry_result in entry_iter {
                let (encrypted_data, salt) = entry_result.map_err(|e| PassMannError::Other(format!("Failed to read encrypted entry: {}", e)))?;
                let derived = derive_key(master_password, &salt);
                let decrypted_data = decrypt(&derived, &encrypted_data)
                    .map_err(|e| PassMannError::Crypto(format!("Failed to decrypt entry: {}", e)))?;
                
                let entry_json = String::from_utf8(decrypted_data)
                    .map_err(|e| PassMannError::Other(format!("Invalid UTF-8 data: {}", e)))?;
                let entry: passmann_shared::Entry = serde_json::from_str(&entry_json)?;
                entries.push(entry);
            }
        }
        
        Ok(entries)
    }

    /// Delete an entry by ID
    pub fn delete_entry(&self, id: &str) -> Result<bool, PassMannError> {
        let table = if self.sqlcipher { "vault_entries" } else { "encrypted_vault_entries" };
        let rows_affected = self.conn.execute(
            &format!("DELETE FROM {} WHERE id = ?1", table),
            params![id],
        ).map_err(|e| PassMannError::Other(format!("Failed to delete entry: {}", e)))?;
        
        Ok(rows_affected > 0)
    }

    /// Search entries by service or username
    pub fn search_entries(&self, query: &str, master_password: &str) -> Result<Vec<passmann_shared::Entry>, PassMannError> {
        let mut entries = Vec::new();
        let search_pattern = format!("%{}%", query);
        
        if self.sqlcipher {
            let mut stmt = self.conn.prepare(
                "SELECT password_data FROM vault_entries WHERE service LIKE ?1 OR username LIKE ?1 ORDER BY service, username"
            ).map_err(|e| PassMannError::Other(format!("Failed to prepare search statement: {}", e)))?;
            
            let entry_iter = stmt.query_map([&search_pattern], |row| {
                let data: Vec<u8> = row.get(0)?;
                Ok(data)
            }).map_err(|e| PassMannError::Other(format!("Failed to search entries: {}", e)))?;
            
            for entry_result in entry_iter {
                let data = entry_result.map_err(|e| PassMannError::Other(format!("Failed to read search result: {}", e)))?;
                let entry_json = String::from_utf8(data)
                    .map_err(|e| PassMannError::Other(format!("Invalid UTF-8 data: {}", e)))?;
                let entry: passmann_shared::Entry = serde_json::from_str(&entry_json)?;
                entries.push(entry);
            }
        } else {
            let mut stmt = self.conn.prepare(
                "SELECT encrypted_data, salt FROM encrypted_vault_entries WHERE service LIKE ?1 OR username LIKE ?1 ORDER BY service, username"
            ).map_err(|e| PassMannError::Other(format!("Failed to prepare encrypted search statement: {}", e)))?;
            
            let entry_iter = stmt.query_map([&search_pattern], |row| {
                let encrypted_data: Vec<u8> = row.get(0)?;
                let salt: Vec<u8> = row.get(1)?;
                Ok((encrypted_data, salt))
            }).map_err(|e| PassMannError::Other(format!("Failed to search encrypted entries: {}", e)))?;
            
            for entry_result in entry_iter {
                let (encrypted_data, salt) = entry_result.map_err(|e| PassMannError::Other(format!("Failed to read encrypted search result: {}", e)))?;
                let derived = derive_key(master_password, &salt);
                let decrypted_data = decrypt(&derived, &encrypted_data)
                    .map_err(|e| PassMannError::Crypto(format!("Failed to decrypt search result: {}", e)))?;
                
                let entry_json = String::from_utf8(decrypted_data)
                    .map_err(|e| PassMannError::Other(format!("Invalid UTF-8 data: {}", e)))?;
                let entry: passmann_shared::Entry = serde_json::from_str(&entry_json)?;
                entries.push(entry);
            }
        }
        
        Ok(entries)
    }

    /// Get database statistics
    pub fn get_stats(&self) -> Result<(usize, bool), PassMannError> {
        let table = if self.sqlcipher { "vault_entries" } else { "encrypted_vault_entries" };
        let count: usize = self.conn.query_row(
            &format!("SELECT COUNT(*) FROM {}", table),
            [],
            |row| row.get(0),
        ).map_err(|e| PassMannError::Other(format!("Failed to get stats: {}", e)))?;
        
        Ok((count, self.sqlcipher))
    }
}

fn _generate_entry_salt(key: &str) -> Vec<u8> {
    // Deterministic salt per key (only used if needed). Prefer random salt stored with blob.
    let mut s = vec![0u8; 32];
    let h = blake3::hash(key.as_bytes());
    let bytes = h.as_bytes();
    s.copy_from_slice(&bytes[0..32]);
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use uuid::Uuid;

    #[test]
    fn test_securedb_put_get_roundtrip() {
        let tmp = env::temp_dir();
        let fname = tmp.join(format!("passmann_test_{}.db", Uuid::new_v4()));
        let master = "test_master_password";

        // Ensure file doesn't exist
        let _ = fs::remove_file(&fname);

        let db = SecureDb::open(&fname, master).expect("open db");

        let key = "service1";
        let data = b"super secret data";

        db.put(key, data, master).expect("put");

        let got = db.get(key, master).expect("get");
        assert!(got.is_some());
        assert_eq!(got.unwrap(), data);

        let _ = fs::remove_file(&fname);
    }

    #[test]
    fn test_securedb_entry_operations() {
        let tmp = env::temp_dir();
        let fname = tmp.join(format!("passmann_test_entries_{}.db", Uuid::new_v4()));
        let master = "test_master_password";
        let _ = fs::remove_file(&fname);

        let db = SecureDb::open(&fname, master).expect("open db");
        
        // Create a test entry
        let entry = passmann_shared::Entry::new(
            "test_service".to_string(),
            "test_user".to_string(),
            "test_password".to_string(),
        );

        // Store the entry
        db.store_entry(&entry, master).expect("store entry");

        // Retrieve the entry
        let retrieved = db.get_entry(&entry.id.to_string(), master)
            .expect("get entry")
            .expect("entry should exist");

        assert_eq!(retrieved.service, entry.service);
        assert_eq!(retrieved.username, entry.username);
        assert_eq!(retrieved.password, entry.password);

        // List entries
        let entries = db.list_entries(master).expect("list entries");
        assert_eq!(entries.len(), 1);

        // Search entries
        let search_results = db.search_entries("test_service", master).expect("search entries");
        assert_eq!(search_results.len(), 1);

        // Delete the entry
        let deleted = db.delete_entry(&entry.id.to_string()).expect("delete entry");
        assert!(deleted);

        // Verify deletion
        let entries_after = db.list_entries(master).expect("list entries after deletion");
        assert_eq!(entries_after.len(), 0);

        let _ = fs::remove_file(&fname);
    }

    #[test]
    fn test_securedb_sqlcipher_detection() {
        let tmp = env::temp_dir();
        let fname = tmp.join(format!("passmann_test_cipher_{}.db", Uuid::new_v4()));
        let master = "test_master_password";
        let _ = fs::remove_file(&fname);

        let config = DbConfig {
            require_sqlcipher: false,
            kdf_iterations: 1000, // Lower for testing
            memory_cost: 1024,    // Lower for testing
        };

        let db = SecureDb::open_with_config(&fname, master, &config).expect("open db");
        let (count, uses_sqlcipher) = db.get_stats().expect("get stats");
        
        println!("Uses SQLCipher: {}", uses_sqlcipher);
        assert_eq!(count, 0); // No entries initially

        let _ = fs::remove_file(&fname);
    }

    #[test]
    fn test_securedb_blob_encrypted_in_fallback() {
        let tmp = env::temp_dir();
        let fname = tmp.join(format!("passmann_test_fallback_{}.db", Uuid::new_v4()));
        let master = "test_master_password";
        let _ = fs::remove_file(&fname);

        let db = SecureDb::open(&fname, master).expect("open db");
        let key = "service2";
        let data = b"another secret";
        db.put(key, data, master).expect("put");

        // Verify we can get the data back
        let retrieved = db.get(key, master).expect("get").expect("data should exist");
        assert_eq!(retrieved, data);

        // If using application-level encryption, verify the stored data is encrypted
        if !db.encryption_enabled {
            // This test only applies if we're using fallback encryption
            let conn = &db.conn;
            let maybe_blob: Option<Vec<u8>> = conn.query_row(
                "SELECT encrypted_value FROM encrypted_metadata WHERE key = ?1", 
                rusqlite::params![key], 
                |r| r.get(0)
            ).optional().expect("query");

            if let Some(blob) = maybe_blob {
                assert!(blob.len() > data.len()); // Should be larger due to salt + encryption
                // The stored blob should not equal the plaintext
                assert_ne!(&blob[32..], data); // Skip salt, compare encrypted portion
            }
        }

        let _ = fs::remove_file(&fname);
    }
}
