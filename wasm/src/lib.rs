use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};
use getrandom::getrandom;
use argon2::Argon2;
use chacha20poly1305::{
    aead::{Aead, KeyInit, generic_array::GenericArray},
    ChaCha20Poly1305, Nonce
};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Entry {
    pub id: String,
    pub service: String,
    pub username: String,
    pub password: String,
    pub url: Option<String>,
    pub notes: Option<String>,
    pub created_at: i64,
    pub modified_at: i64,
    pub is_favorite: bool,
}

#[derive(Serialize, Deserialize)]
pub struct VaultData {
    entries: Vec<Entry>,
    created_at: i64,
    modified_at: i64,
    storage_mode: String,
}

#[wasm_bindgen]
pub struct PassMannWasm {
    master_key: Option<[u8; 32]>,
    vault_data: Option<Vec<Entry>>,
    storage_mode: String,
}

#[wasm_bindgen]
impl PassMannWasm {
    #[wasm_bindgen(constructor)]
    pub fn new() -> PassMannWasm {
        console_error_panic_hook::set_once();
        PassMannWasm {
            master_key: None,
            vault_data: None,
            storage_mode: "local".to_string(),
        }
    }

    #[wasm_bindgen]
    pub fn set_storage_mode(&mut self, mode: &str) {
        self.storage_mode = mode.to_string();
        console_log!("Storage mode set to: {}", mode);
    }

    #[wasm_bindgen]
    pub fn unlock_vault(&mut self, master_password: &str, salt: &[u8], encrypted_vault: Option<Vec<u8>>) -> bool {
        if salt.len() < 16 {
            console_log!("Salt too short");
            return false;
        }
        
        let key = match self.derive_key(master_password, salt) {
            Ok(k) => k,
            Err(e) => {
                console_log!("Key derivation failed: {}", e);
                return false;
            }
        };
        self.master_key = Some(key);
        
        // If we have encrypted vault data, decrypt it
        if let Some(encrypted_data) = encrypted_vault {
            match self.decrypt_vault_data(&encrypted_data) {
                Some(entries) => {
                    self.vault_data = Some(entries);
                    console_log!("Vault unlocked with {} entries", self.vault_data.as_ref().unwrap().len());
                }
                None => {
                    console_log!("Failed to decrypt vault data");
                    return false;
                }
            }
        } else {
            // New vault
            self.vault_data = Some(Vec::new());
            console_log!("New vault created");
        }
        
        true
    }

    #[wasm_bindgen]
    pub fn add_entry(&mut self, service: &str, username: &str, password: &str, url: Option<String>, notes: Option<String>) -> bool {
        if let Some(entries) = &mut self.vault_data {
            let now = chrono::Utc::now().timestamp_millis();
            let entry = Entry {
                id: format!("entry_{}", now),
                service: service.to_string(),
                username: username.to_string(),
                password: password.to_string(),
                url,
                notes,
                created_at: now,
                modified_at: now,
                is_favorite: false,
            };
            entries.push(entry);
            console_log!("Entry added for service: {}", service);
            true
        } else {
            console_log!("Vault not unlocked");
            false
        }
    }

    #[wasm_bindgen]
    pub fn update_entry(&mut self, index: usize, service: &str, username: &str, password: &str, url: Option<String>, notes: Option<String>) -> bool {
        if let Some(entries) = &mut self.vault_data {
            if index < entries.len() {
                let entry = &mut entries[index];
                entry.service = service.to_string();
                entry.username = username.to_string();
                entry.password = password.to_string();
                entry.url = url;
                entry.notes = notes;
                entry.modified_at = chrono::Utc::now().timestamp_millis();
                console_log!("Entry updated for service: {}", service);
                true
            } else {
                console_log!("Entry index out of bounds");
                false
            }
        } else {
            console_log!("Vault not unlocked");
            false
        }
    }

    #[wasm_bindgen]
    pub fn delete_entry(&mut self, index: usize) -> bool {
        if let Some(entries) = &mut self.vault_data {
            if index < entries.len() {
                let removed = entries.remove(index);
                console_log!("Entry deleted for service: {}", removed.service);
                true
            } else {
                console_log!("Entry index out of bounds");
                false
            }
        } else {
            console_log!("Vault not unlocked");
            false
        }
    }

    #[wasm_bindgen]
    pub fn get_entries_json(&self) -> Option<String> {
        if let Some(entries) = &self.vault_data {
            match serde_json::to_string(entries) {
                Ok(json) => Some(json),
                Err(e) => {
                    console_log!("Failed to serialize entries: {}", e);
                    None
                }
            }
        } else {
            console_log!("Vault not unlocked");
            None
        }
    }

    #[wasm_bindgen]
    pub fn search_entries(&self, query: &str) -> Option<String> {
        if let Some(entries) = &self.vault_data {
            let query_lower = query.to_lowercase();
            let filtered: Vec<&Entry> = entries
                .iter()
                .filter(|entry| {
                    entry.service.to_lowercase().contains(&query_lower) ||
                    entry.username.to_lowercase().contains(&query_lower) ||
                    entry.url.as_ref().map_or(false, |url| url.to_lowercase().contains(&query_lower)) ||
                    entry.notes.as_ref().map_or(false, |notes| notes.to_lowercase().contains(&query_lower))
                })
                .collect();
            
            match serde_json::to_string(&filtered) {
                Ok(json) => Some(json),
                Err(e) => {
                    console_log!("Failed to serialize search results: {}", e);
                    None
                }
            }
        } else {
            None
        }
    }

    #[wasm_bindgen]
    pub fn encrypt_vault(&self) -> Option<Vec<u8>> {
        if let (Some(key), Some(entries)) = (&self.master_key, &self.vault_data) {
            let now = chrono::Utc::now().timestamp_millis();
            let vault_data = VaultData {
                entries: entries.clone(),
                created_at: now,
                modified_at: now,
                storage_mode: self.storage_mode.clone(),
            };
            
            match serde_json::to_string(&vault_data) {
                Ok(json) => {
                    match self.encrypt_data_internal(key, json.as_bytes()) {
                        Ok(encrypted) => Some(encrypted),
                        Err(e) => {
                            console_log!("Encryption failed: {}", e);
                            None
                        }
                    }
                }
                Err(e) => {
                    console_log!("Failed to serialize vault: {}", e);
                    None
                }
            }
        } else {
            console_log!("Vault not ready for encryption");
            None
        }
    }

    fn decrypt_vault_data(&self, encrypted_data: &[u8]) -> Option<Vec<Entry>> {
        if let Some(key) = &self.master_key {
            match self.decrypt_data_internal(key, encrypted_data) {
                Ok(decrypted) => {
                    match String::from_utf8(decrypted) {
                        Ok(json) => {
                            match serde_json::from_str::<VaultData>(&json) {
                                Ok(vault_data) => Some(vault_data.entries),
                                Err(e) => {
                                    console_log!("Failed to deserialize vault: {}", e);
                                    None
                                }
                            }
                        }
                        Err(e) => {
                            console_log!("UTF-8 conversion failed: {}", e);
                            None
                        }
                    }
                }
                Err(e) => {
                    console_log!("Decryption failed: {}", e);
                    None
                }
            }
        } else {
            console_log!("Master key not available");
            None
        }
    }

    // Crypto functions
    fn derive_key(&self, password: &str, salt: &[u8]) -> Result<[u8; 32], String> {
        let argon2 = Argon2::default();
        let mut output = [0u8; 32];
        
        argon2.hash_password_into(password.as_bytes(), salt, &mut output)
            .map_err(|e| format!("Argon2 error: {}", e))?;
        
        Ok(output)
    }

    fn encrypt_data_internal(&self, key: &[u8; 32], data: &[u8]) -> Result<Vec<u8>, String> {
        let cipher = ChaCha20Poly1305::new(GenericArray::from_slice(key));
        let mut nonce_bytes = [0u8; 12];
        getrandom(&mut nonce_bytes).map_err(|e| format!("Random generation failed: {}", e))?;
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        let ciphertext = cipher.encrypt(nonce, data)
            .map_err(|e| format!("Encryption failed: {}", e))?;
        
        let mut result = nonce_bytes.to_vec();
        result.extend_from_slice(&ciphertext);
        Ok(result)
    }

    fn decrypt_data_internal(&self, key: &[u8; 32], encrypted_data: &[u8]) -> Result<Vec<u8>, String> {
        if encrypted_data.len() < 12 {
            return Err("Invalid encrypted data".to_string());
        }
        
        let cipher = ChaCha20Poly1305::new(GenericArray::from_slice(key));
        let nonce = Nonce::from_slice(&encrypted_data[0..12]);
        let ciphertext = &encrypted_data[12..];
        
        cipher.decrypt(nonce, ciphertext)
            .map_err(|e| format!("Decryption failed: {}", e))
    }

    #[wasm_bindgen]
    pub fn unlock(&mut self, master_password: &str, salt: &[u8]) -> bool {
        if salt.len() < 16 {
            console_log!("Salt too short");
            return false;
        }
        
        match self.derive_key(master_password, salt) {
            Ok(key) => {
                self.master_key = Some(key);
                console_log!("Successfully unlocked vault");
                true
            }
            Err(e) => {
                console_log!("Failed to unlock: {}", e);
                false
            }
        }
    }

    #[wasm_bindgen]
    pub fn encrypt_data(&self, data: &str) -> Option<Vec<u8>> {
        if let Some(key) = &self.master_key {
            match self.encrypt_data_internal(key, data.as_bytes()) {
                Ok(encrypted) => Some(encrypted),
                Err(e) => {
                    console_log!("Encryption failed: {}", e);
                    None
                }
            }
        } else {
            console_log!("Vault not unlocked");
            None
        }
    }

    #[wasm_bindgen]
    pub fn decrypt_data(&self, encrypted_data: &[u8]) -> Option<String> {
        if let Some(key) = &self.master_key {
            match self.decrypt_data_internal(key, encrypted_data) {
                Ok(decrypted) => {
                    match String::from_utf8(decrypted) {
                        Ok(text) => Some(text),
                        Err(e) => {
                            console_log!("UTF-8 conversion failed: {}", e);
                            None
                        }
                    }
                }
                Err(e) => {
                    console_log!("Decryption failed: {}", e);
                    None
                }
            }
        } else {
            console_log!("Vault not unlocked");
            None
        }
    }

    #[wasm_bindgen]
    pub fn generate_salt() -> Vec<u8> {
        let mut salt = [0u8; 32];
        getrandom(&mut salt).unwrap_or_else(|_| {
            console_log!("Warning: getrandom failed, using timestamp fallback");
        });
        salt.to_vec()
    }

    #[wasm_bindgen]
    pub fn is_unlocked(&self) -> bool {
        self.master_key.is_some()
    }

    #[wasm_bindgen]
    pub fn lock(&mut self) {
        self.master_key = None;
        console_log!("Vault locked");
    }

    // Cloud sync functionality
    #[wasm_bindgen]
    pub fn set_cloud_mode(&mut self, server_url: &str) {
        self.storage_mode = "cloud".to_string();
        console_log!("Cloud mode enabled with server: {}", server_url);
    }

    #[wasm_bindgen]
    pub fn get_storage_mode(&self) -> String {
        self.storage_mode.clone()
    }

    #[wasm_bindgen]
    pub fn get_entries_count(&self) -> usize {
        self.vault_data.as_ref().map_or(0, |entries| entries.len())
    }
}

// Initialize WASM module
#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    console_log!("PassMann WASM module initialized");
}

