use crate::entry::Entry;
use crate::crypto::{encrypt, decrypt, derive_key};
use crate::security::{VaultLock, AuditLog};
use serde::{Serialize, Deserialize};
use dirs;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use zeroize::Zeroize;
use getrandom::getrandom;

#[derive(Serialize, Deserialize)]
pub struct Vault {
    pub entries: Vec<Entry>,
    #[serde(skip)]
    pub lock: Option<VaultLock>,
    #[serde(skip)]  
    pub audit: Option<AuditLog>,
    // Store salt with the vault for better security
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vault_salt: Option<Vec<u8>>,
}

impl Vault {
    pub fn new(timeout_secs: u64) -> Self {
        Self { 
            entries: Vec::new(),
            lock: Some(VaultLock::new(timeout_secs)),
            audit: Some(AuditLog::new()),
            vault_salt: None,
        }
    }

    pub fn add_entry(&mut self, service: String, username: String, password: String) {
        if self.check_and_handle_lock() {
            println!("❌ Vault is locked due to inactivity. Please restart the application.");
            return;
        }

        // Validate inputs
        if service.trim().is_empty() || username.trim().is_empty() {
            println!("❌ Service and username cannot be empty!");
            return;
        }

        // Use Entry::new to ensure all fields are initialized correctly
        self.entries.push(Entry::new(
            service.trim().to_string(),
            username.trim().to_string(),
            password,
        ));
        
        if let Some(ref audit) = self.audit {
            audit.log(&format!("Entry added for service: {}", service));
        }
        if let Some(ref lock) = self.lock {
            lock.refresh();
        }
    }

    pub fn get_entries(&self) -> Option<&Vec<Entry>> {
        if self.check_and_handle_lock() {
            println!("❌ Vault is locked due to inactivity. Please restart the application.");
            return None;
        }
        
        if let Some(ref lock) = self.lock {
            lock.refresh();
        }
        Some(&self.entries)
    }

    pub fn remove_entries(&mut self, service_filter: &str) -> usize {
        if self.check_and_handle_lock() {
            println!("❌ Vault is locked due to inactivity. Please restart the application.");
            return 0;
        }

        let before_count = self.entries.len();
        self.entries.retain(|e| !e.service.to_lowercase().contains(&service_filter.to_lowercase()));
        let removed_count = before_count - self.entries.len();
        
        if removed_count > 0 {
            if let Some(ref audit) = self.audit {
                audit.log(&format!("Removed {} entries matching '{}'", removed_count, service_filter));
            }
            if let Some(ref lock) = self.lock {
                lock.refresh();
            }
        }
        
        removed_count
    }

    pub fn save(&mut self, master_password: &str) -> std::io::Result<()> {
        // Generate or use existing salt for this vault
        let salt = match &self.vault_salt {
            Some(existing_salt) => existing_salt.clone(),
                None => {
                let mut new_salt = vec![0u8; 32]; // 32 bytes for better security
                getrandom(&mut new_salt).expect("OS RNG failed");
                self.vault_salt = Some(new_salt.clone());
                new_salt
            }
        };

        let mut key = derive_key(master_password, &salt);
        
        // Create a save structure that includes the salt
        #[derive(Serialize)]
        struct VaultSave {
            salt: Vec<u8>,
            entries: Vec<Entry>,
        }
        
        let save_data = VaultSave {
            salt: salt.clone(),
            entries: self.entries.clone(),
        };
        
        let data = serde_json::to_vec(&save_data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let encrypted = encrypt(&key, &data);
        let vault_path = Self::vault_path()?;
        
        // Create directory if it doesn't exist
        if let Some(parent) = vault_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        // Write atomically by writing to temp file first
        let temp_path = vault_path.with_extension("tmp");
        let mut file = File::create(&temp_path)?;
        file.write_all(&encrypted)?;
        file.sync_all()?; // Ensure data is written to disk
        
        // Atomic rename
        std::fs::rename(temp_path, vault_path)?;
        
        key.zeroize();

        if let Some(ref audit) = self.audit {
            audit.log("Vault saved to disk securely");
        }
        
        Ok(())
    }

    pub fn load(master_password: &str) -> std::io::Result<Self> {
        let vault_path = Self::vault_path()?;
        
        if !vault_path.exists() {
        let v = Self::new(900); // 15 minute timeout
            if let Some(ref audit) = v.audit {
                audit.log("New vault created");
            }
            return Ok(v);
        }
        
        let encrypted = fs::read(vault_path)?;
        
        // Try with different salt strategies for backward compatibility
        let mut vault_data = None;
        
        // First, try to decrypt assuming it contains salt
        if let Ok(data) = Self::try_decrypt_with_embedded_salt(master_password, &encrypted) {
            vault_data = Some(data);
        } 
        // Fallback to legacy static salt for existing vaults
        else if let Ok(data) = Self::try_decrypt_with_legacy_salt(master_password, &encrypted) {
            vault_data = Some(data);
        }
        
        match vault_data {
            Some((entries, salt)) => {
                let v = Self::new(900);
                let vault = Self {
                    entries,
                    lock: v.lock,
                    audit: v.audit,
                    vault_salt: Some(salt),
                };
                
                if let Some(ref audit) = vault.audit {
                    audit.log("Vault loaded from disk successfully");
                }
                Ok(vault)
            }
            None => {
                Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData, 
                    "Failed to decrypt vault - invalid master password or corrupted file"
                ))
            }
        }
    }

    fn try_decrypt_with_embedded_salt(master_password: &str, encrypted: &[u8]) -> Result<(Vec<Entry>, Vec<u8>), Box<dyn std::error::Error>> {
        #[derive(Deserialize)]
        struct VaultSave {
            salt: Vec<u8>,
            entries: Vec<Entry>,
        }
        
        // Try to decrypt with a temporary salt first to see if it's the new format
        let temp_salt = vec![0u8; 32];
        let mut key = derive_key(master_password, &temp_salt);
        
        if let Ok(data) = decrypt(&key, encrypted) {
            if let Ok(save_data) = serde_json::from_slice::<VaultSave>(&data) {
                key.zeroize();
                // Re-derive with the actual salt
                let mut real_key = derive_key(master_password, &save_data.salt);
                let real_data = decrypt(&real_key, encrypted)?;
                let real_save_data: VaultSave = serde_json::from_slice(&real_data)?;
                real_key.zeroize();
                return Ok((real_save_data.entries, real_save_data.salt));
            }
        }
        
        key.zeroize();
        Err("Not new format".into())
    }

    fn try_decrypt_with_legacy_salt(master_password: &str, encrypted: &[u8]) -> Result<(Vec<Entry>, Vec<u8>), Box<dyn std::error::Error>> {
        let legacy_salt = b"UniqueAppSaltV1Secure2024";
        let mut key = derive_key(master_password, legacy_salt);
        let data = decrypt(&key, encrypted)?;
        let entries: Vec<Entry> = serde_json::from_slice(&data)?;
        key.zeroize();
        Ok((entries, legacy_salt.to_vec()))
    }

    fn vault_path() -> std::io::Result<PathBuf> {
        let mut path = dirs::data_local_dir()
            .or_else(|| dirs::home_dir().map(|h| h.join(".local").join("share")))
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        
        path.push("PassMan");
        
        // Ensure the directory exists with proper permissions
        if !path.exists() {
            std::fs::create_dir_all(&path)?;
            
            // Set restrictive permissions on Unix-like systems
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = std::fs::metadata(&path)?.permissions();
                perms.set_mode(0o700); // Owner read/write/execute only
                std::fs::set_permissions(&path, perms)?;
            }
        }
        
        path.push("vault.enc");
        Ok(path)
    }

    pub fn check_and_handle_lock(&self) -> bool {
        if let Some(ref lock) = self.lock {
            if lock.is_locked() {
                if let Some(ref audit) = self.audit {
                    audit.log("Vault auto-locked due to inactivity timeout");
                }
                return true;
            }
        }
        false
    }

    pub fn get_lock_status(&self) -> Option<std::time::Duration> {
        if let Some(ref lock) = self.lock {
            Some(lock.time_until_lock())
        } else {
            None
        }
    }

    pub fn persist_audit_log(&self) -> std::io::Result<()> {
        if let Some(ref audit) = self.audit {
            let vault_path = Self::vault_path()?;
            let mut log_path = vault_path.clone();
            log_path.set_file_name("audit.log");
            audit.persist(&log_path.to_string_lossy())?;
        }
        Ok(())
    }

    pub fn get_vault_stats(&self) -> VaultStats {
        VaultStats {
            total_entries: self.entries.len(),
            unique_services: self.entries.iter()
                .map(|e| e.service.as_str())
                .collect::<std::collections::HashSet<_>>()
                .len(),
            has_duplicates: self.has_duplicate_services(),
        }
    }

    fn has_duplicate_services(&self) -> bool {
        let mut services = std::collections::HashSet::new();
        self.entries.iter().any(|e| !services.insert(&e.service))
    }

    pub fn export_entries(&self, format: &str) -> Result<String, Box<dyn std::error::Error>> {
        match format.to_lowercase().as_str() {
            "json" => Ok(serde_json::to_string_pretty(&self.entries)?),
            "csv" => {
                let mut csv = String::from("Service,Username,Password\n");
                for entry in &self.entries {
                    csv.push_str(&format!("{},{},{}\n", 
                        entry.service.replace(",", "\\,"),
                        entry.username.replace(",", "\\,"),
                        entry.password.replace(",", "\\,")
                    ));
                }
                Ok(csv)
            }
            _ => Err("Unsupported format. Use 'json' or 'csv'".into())
        }
    }
}

#[derive(Debug)]
pub struct VaultStats {
    pub total_entries: usize,
    pub unique_services: usize,
    pub has_duplicates: bool,
}
