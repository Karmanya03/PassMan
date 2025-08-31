use anyhow::Context;
use chrono::{DateTime, Utc};
use reqwest::{Client, header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE}};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::env;
use uuid::Uuid;
use passmann_shared::Result;

/// Supabase cloud storage client for PassMann
/// Provides secure cloud synchronization with zero-knowledge architecture
#[derive(Debug, Clone)]
pub struct SupabaseClient {
    client: Client,
    base_url: String,
    anon_key: String,
    user_id: Option<String>,
}

/// Encrypted vault data structure for cloud storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudVault {
    pub id: Option<Uuid>,
    pub user_id: String,
    pub encrypted_data: String,
    pub salt: String,
    pub device_id: String,
    pub device_name: String,
    pub version: i32,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub checksum: String,
    pub compression_enabled: bool,
    pub size_bytes: i64,
}

/// Sync metadata for conflict resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncMetadata {
    pub user_id: String,
    pub device_id: String,
    pub last_sync: DateTime<Utc>,
    pub sync_version: i32,
    pub pending_changes: bool,
    pub conflict_resolution: String,
}

/// Audit log entry for security tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub user_id: String,
    pub action: String,
    pub device_id: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub success: bool,
    pub error_message: Option<String>,
    pub metadata: Option<Value>,
}

impl SupabaseClient {
    /// Initialize Supabase client with environment configuration
    pub fn new() -> Result<Self> {
        dotenv::dotenv().ok(); // Load .env file if present
        
        let base_url = env::var("SUPABASE_URL")
            .context("SUPABASE_URL environment variable not set")?;
        let anon_key = env::var("SUPABASE_ANON_KEY")
            .context("SUPABASE_ANON_KEY environment variable not set")?;
        
        let client = Client::new();
        
        Ok(Self {
            client,
            base_url,
            anon_key,
            user_id: None,
        })
    }
    
    /// Authenticate user and establish session
    pub async fn authenticate(&mut self, user_id: String) -> Result<&mut Self> {
        self.user_id = Some(user_id.clone());
        
        // Verify user exists or create profile
        self.ensure_user_profile(user_id).await?;
        
        Ok(self)
    }
    
    /// Ensure user profile exists in Supabase
    async fn ensure_user_profile(&self, user_id: String) -> Result<()> {
        let url = format!("{}/rest/v1/user_profiles", self.base_url);
        
        let profile_data = json!({
            "user_id": user_id,
            "created_at": Utc::now(),
            "last_login": Utc::now(),
            "device_count": 1,
            "total_vaults": 0,
            "subscription_tier": "free"
        });
        
        let response = self.client
            .post(&url)
            .headers(self.get_headers()?)
            .json(&profile_data)
            .send()
            .await
            .context("Failed to create user profile")?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            log::debug!("User profile creation response: {}", error_text);
            // Profile might already exist, which is fine
        }
        
        Ok(())
    }
    
    /// Upload encrypted vault to cloud storage
    pub async fn upload_vault(&self, vault: &CloudVault) -> Result<Uuid> {
        let _user_id = self.user_id.as_ref()
            .context("Must authenticate before uploading vault")?;
        
        let url = format!("{}/rest/v1/encrypted_vaults", self.base_url);
        
        let response = self.client
            .post(&url)
            .headers(self.get_headers()?)
            .json(vault)
            .send()
            .await
            .context("Failed to upload vault to Supabase")?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("Vault upload failed: {}", error_text).into());
        }
        
        let result: Vec<CloudVault> = response.json().await
            .context("Failed to parse upload response")?;
        
        let vault_id = result.first()
            .and_then(|v| v.id)
            .context("No vault ID returned from upload")?;
        
        // Log successful upload
        self.log_audit_action("vault_upload", true, None, Some(json!({
            "vault_id": vault_id,
            "size_bytes": vault.size_bytes
        }))).await?;
        
        Ok(vault_id)
    }
    
    /// Download encrypted vault from cloud storage
    pub async fn download_vault(&self, device_id: &str) -> Result<Option<CloudVault>> {
        let user_id = self.user_id.as_ref()
            .context("Must authenticate before downloading vault")?;
        
        let url = format!("{}/rest/v1/encrypted_vaults", self.base_url);
        
        let response = self.client
            .get(&url)
            .headers(self.get_headers()?)
            .query(&[
                ("user_id", format!("eq.{}", user_id)),
                ("device_id", format!("eq.{}", device_id)),
                ("order", "updated_at.desc".to_string()),
                ("limit", "1".to_string())
            ])
            .send()
            .await
            .context("Failed to download vault from Supabase")?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("Vault download failed: {}", error_text).into());
        }
        
        let vaults: Vec<CloudVault> = response.json().await
            .context("Failed to parse download response")?;
        
        let vault = vaults.into_iter().next();
        
        // Log successful download
        if vault.is_some() {
            self.log_audit_action("vault_download", true, None, Some(json!({
                "device_id": device_id
            }))).await?;
        }
        
        Ok(vault)
    }
    
    /// Update existing vault in cloud storage
    #[allow(dead_code)]
    pub async fn update_vault(&self, vault_id: Uuid, vault: &CloudVault) -> Result<()> {
        let _user_id = self.user_id.as_ref()
            .context("Must authenticate before updating vault")?;
        
        let url = format!("{}/rest/v1/encrypted_vaults", self.base_url);
        
        let response = self.client
            .patch(&url)
            .headers(self.get_headers()?)
            .query(&[("id", format!("eq.{}", vault_id))])
            .json(vault)
            .send()
            .await
            .context("Failed to update vault in Supabase")?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("Vault update failed: {}", error_text).into());
        }
        
        // Log successful update
        self.log_audit_action("vault_update", true, None, Some(json!({
            "vault_id": vault_id,
            "size_bytes": vault.size_bytes
        }))).await?;
        
        Ok(())
    }
    
    /// Get sync metadata for conflict resolution
    pub async fn get_sync_metadata(&self, device_id: &str) -> Result<Option<SyncMetadata>> {
        let user_id = self.user_id.as_ref()
            .context("Must authenticate before getting sync metadata")?;
        
        let url = format!("{}/rest/v1/sync_metadata", self.base_url);
        
        let response = self.client
            .get(&url)
            .headers(self.get_headers()?)
            .query(&[
                ("user_id", format!("eq.{}", user_id)),
                ("device_id", format!("eq.{}", device_id))
            ])
            .send()
            .await
            .context("Failed to get sync metadata from Supabase")?;
        
        if !response.status().is_success() {
            return Ok(None);
        }
        
        let metadata: Vec<SyncMetadata> = response.json().await
            .context("Failed to parse sync metadata response")?;
        
        Ok(metadata.into_iter().next())
    }
    
    /// Update sync metadata after successful sync
    pub async fn update_sync_metadata(&self, metadata: &SyncMetadata) -> Result<()> {
        let url = format!("{}/rest/v1/sync_metadata", self.base_url);
        
        let response = self.client
            .post(&url)
            .headers(self.get_headers()?)
            .json(metadata)
            .send()
            .await
            .context("Failed to update sync metadata")?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            log::warn!("Sync metadata update failed: {}", error_text);
        }
        
        Ok(())
    }
    
    /// Log audit action for security tracking
    pub async fn log_audit_action(
        &self,
        action: &str,
        success: bool,
        error_message: Option<String>,
        metadata: Option<Value>
    ) -> Result<()> {
        let user_id = self.user_id.as_ref()
            .context("Must authenticate before logging audit action")?;
        
        let device_id = env::var("PASSMANN_DEVICE_ID")
            .unwrap_or_else(|_| "unknown".to_string());
        
        let audit_log = AuditLog {
            user_id: user_id.clone(),
            action: action.to_string(),
            device_id,
            ip_address: None, // Could be added via external API
            user_agent: Some("PassMann Desktop".to_string()),
            success,
            error_message,
            metadata,
        };
        
        let url = format!("{}/rest/v1/audit_logs", self.base_url);
        
        let response = self.client
            .post(&url)
            .headers(self.get_headers()?)
            .json(&audit_log)
            .send()
            .await;
        
        if let Err(e) = response {
            log::warn!("Failed to log audit action: {}", e);
        }
        
        Ok(())
    }
    
    /// Get audit logs for security monitoring
    pub async fn get_audit_logs(&self, limit: Option<i32>) -> Result<Vec<AuditLog>> {
        let user_id = self.user_id.as_ref()
            .context("Must authenticate before getting audit logs")?;
        
        let url = format!("{}/rest/v1/audit_logs", self.base_url);
        let limit_str = limit.unwrap_or(50).to_string();
        
        let response = self.client
            .get(&url)
            .headers(self.get_headers()?)
            .query(&[
                ("user_id", format!("eq.{}", user_id)),
                ("order", "created_at.desc".to_string()),
                ("limit", limit_str)
            ])
            .send()
            .await
            .context("Failed to get audit logs from Supabase")?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("Audit logs fetch failed: {}", error_text).into());
        }
        
        let logs: Vec<AuditLog> = response.json().await
            .context("Failed to parse audit logs response")?;
        
        Ok(logs)
    }
    
    /// Delete vault from cloud storage
    #[allow(dead_code)]
    pub async fn delete_vault(&self, vault_id: Uuid) -> Result<()> {
        let url = format!("{}/rest/v1/encrypted_vaults", self.base_url);
        
        let response = self.client
            .delete(&url)
            .headers(self.get_headers()?)
            .query(&[("id", format!("eq.{}", vault_id))])
            .send()
            .await
            .context("Failed to delete vault from Supabase")?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("Vault deletion failed: {}", error_text).into());
        }
        
        // Log successful deletion
        self.log_audit_action("vault_delete", true, None, Some(json!({
            "vault_id": vault_id
        }))).await?;
        
        Ok(())
    }
    
    /// Generate HTTP headers for Supabase API requests
    fn get_headers(&self) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();
        
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", self.anon_key))
                .context("Invalid authorization header")?
        );
        
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/json")
        );
        
        headers.insert(
            "apikey",
            HeaderValue::from_str(&self.anon_key)
                .context("Invalid API key header")?
        );
        
        Ok(headers)
    }
}
