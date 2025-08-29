#![allow(dead_code)]
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc, Duration};
use std::collections::HashMap;

/// Password entry with comprehensive security features and metadata
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Entry {
    /// Unique identifier for the entry
    #[serde(default = "generate_entry_id")]
    pub id: String,
    
    /// Service name (e.g., gmail, github, banking)
    pub service: String,
    
    /// Username, email, or account identifier
    pub username: String,
    
    /// The actual password (encrypted at rest)
    pub password: String,
    
    /// Optional URL associated with the service
    pub url: Option<String>,
    
    /// Secure notes and additional information
    pub notes: Option<String>,
    
    /// Entry category for organization
    #[serde(default = "default_category")]
    pub category: EntryCategory,
    
    /// Custom tags for flexible organization
    #[serde(default)]
    pub tags: Vec<String>,
    
    /// Custom fields for additional data
    #[serde(default)]
    pub custom_fields: HashMap<String, String>,
    
    /// Password strength assessment
    #[serde(default)]
    pub password_strength: PasswordStrengthInfo,
    
    /// Timestamp when entry was created
    #[serde(default = "current_timestamp")]
    pub created_at: DateTime<Utc>,
    
    /// Timestamp when entry was last modified
    #[serde(default = "current_timestamp")]
    pub modified_at: DateTime<Utc>,
    
    /// Timestamp when password was last accessed
    pub last_accessed: Option<DateTime<Utc>>,
    
    /// Password expiration settings
    pub expiration: Option<PasswordExpiration>,
    
    /// Password history for tracking changes
    #[serde(default)]
    pub password_history: Vec<PasswordHistoryEntry>,
    
    /// Security metadata and risk assessment
    #[serde(default)]
    pub security_info: SecurityInfo,
    
    /// Entry-specific settings and preferences
    #[serde(default)]
    pub settings: EntrySettings,
    
    /// Favorite/pinned status for quick access
    #[serde(default)]
    pub is_favorite: bool,
    
    /// Soft delete flag (for recovery purposes)
    #[serde(default)]
    pub is_deleted: bool,
    
    /// Deletion timestamp (if soft deleted)
    pub deleted_at: Option<DateTime<Utc>>,
}

/// Entry categories for organization
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum EntryCategory {
    Personal,
    Work,
    Finance,
    Social,
    Gaming,
    Shopping,
    Education,
    Healthcare,
    Government,
    Development,
    Other(String),
}

/// Password strength information
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct PasswordStrengthInfo {
    pub score: u32,           // 0-100 strength score
    pub level: String,        // Weak, Fair, Good, Strong
    pub entropy: f64,         // Calculated entropy
    pub has_lowercase: bool,
    pub has_uppercase: bool,
    pub has_numbers: bool,
    pub has_symbols: bool,
    pub length: usize,
    pub is_common: bool,      // Detected as common password
    pub is_compromised: bool, // Found in breach databases
    pub feedback: Vec<String>, // Improvement suggestions
    pub last_checked: DateTime<Utc>,
}

/// Password expiration configuration
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PasswordExpiration {
    pub expires_at: DateTime<Utc>,
    pub warning_days: u32,          // Days before expiration to warn
    pub auto_remind: bool,          // Send automatic reminders
    pub policy_source: String,      // Source of expiration policy
}

/// Password history entry for tracking changes
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PasswordHistoryEntry {
    pub password_hash: String,      // Hashed version for comparison
    pub changed_at: DateTime<Utc>,
    pub changed_reason: String,     // Manual, Expired, Compromised, etc.
    pub strength_score: u32,
}

/// Security metadata and risk assessment
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct SecurityInfo {
    pub risk_level: RiskLevel,
    pub threat_indicators: Vec<ThreatIndicator>,
    pub last_security_check: Option<DateTime<Utc>>,
    pub breach_check_status: BreachCheckStatus,
    pub two_factor_enabled: Option<bool>,
    pub security_questions: u32,    // Number of security questions set
    pub account_recovery_methods: Vec<String>,
    pub suspicious_activity_detected: bool,
    pub last_known_good_access: Option<DateTime<Utc>>,
}

/// Risk assessment levels
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub enum RiskLevel {
    #[default]
    Unknown,
    Low,
    Medium,
    High,
    Critical,
}

/// Security threat indicators
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ThreatIndicator {
    pub indicator_type: String,     // WeakPassword, DataBreach, SuspiciousLogin
    pub severity: String,           // Low, Medium, High, Critical  
    pub description: String,
    pub detected_at: DateTime<Utc>,
    pub source: String,             // HaveIBeenPwned, InternalCheck, etc.
}

/// Breach check status
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct BreachCheckStatus {
    pub last_checked: Option<DateTime<Utc>>,
    pub is_compromised: bool,
    pub breach_count: u32,
    pub most_recent_breach: Option<DateTime<Utc>>,
    pub breach_sources: Vec<String>,
}

/// Entry-specific settings
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct EntrySettings {
    pub auto_fill_enabled: bool,
    pub require_master_password: bool,  // Require re-auth for access
    pub clipboard_timeout: Option<u32>, // Seconds before clipboard clear
    pub hide_password_by_default: bool,
    pub enable_breach_monitoring: bool,
    pub password_change_reminders: bool,
    pub access_logging_enabled: bool,
}

impl Entry {
    /// Create a new password entry with security defaults
    pub fn new(service: String, username: String, password: String) -> Self {
        let mut entry = Self {
            id: generate_entry_id(),
            service: service.trim().to_string(),
            username: username.trim().to_string(),
            password: password.clone(),
            url: None,
            notes: None,
            category: EntryCategory::Personal,
            tags: Vec::new(),
            custom_fields: HashMap::new(),
            password_strength: PasswordStrengthInfo::default(),
            created_at: Utc::now(),
            modified_at: Utc::now(),
            last_accessed: None,
            expiration: None,
            password_history: Vec::new(),
            security_info: SecurityInfo::default(),
            settings: EntrySettings::default(),
            is_favorite: false,
            is_deleted: false,
            deleted_at: None,
        };

        // Immediately assess password strength
        entry.update_password_strength(&password);
        entry
    }

    /// Update password with history tracking
    pub fn update_password(&mut self, new_password: String, reason: &str) {
        // Add current password to history
        if !self.password.is_empty() {
            let history_entry = PasswordHistoryEntry {
                password_hash: self.hash_password(&self.password),
                changed_at: Utc::now(),
                changed_reason: reason.to_string(),
                strength_score: self.password_strength.score,
            };
            self.password_history.push(history_entry);
            
            // Keep only last 10 passwords
            if self.password_history.len() > 10 {
                self.password_history.remove(0);
            }
        }

        // Update password and metadata
        self.password = new_password.clone();
        self.modified_at = Utc::now();
        self.update_password_strength(&new_password);
        
        // Clear compromised status (will be re-checked)
        self.security_info.breach_check_status.is_compromised = false;
    }

    /// Update password strength assessment
    pub fn update_password_strength(&mut self, password: &str) {
        self.password_strength = Self::assess_password_strength(password);
        self.update_risk_level();
    }

    /// Assess password strength comprehensively
    pub fn assess_password_strength(password: &str) -> PasswordStrengthInfo {
        let mut info = PasswordStrengthInfo {
            length: password.len(),
            has_lowercase: password.chars().any(|c| c.is_ascii_lowercase()),
            has_uppercase: password.chars().any(|c| c.is_ascii_uppercase()),
            has_numbers: password.chars().any(|c| c.is_ascii_digit()),
            has_symbols: password.chars().any(|c| !c.is_alphanumeric()),
            last_checked: Utc::now(),
            ..Default::default()
        };

        // Calculate entropy
        info.entropy = Self::calculate_entropy(password);

        // Calculate score
        info.score = Self::calculate_strength_score(&info, password);

        // Determine level
        info.level = match info.score {
            0..=30 => "Weak".to_string(),
            31..=60 => "Fair".to_string(),
            61..=80 => "Good".to_string(),
            81..=100 => "Strong".to_string(),
            _ => "Unknown".to_string(),
        };

        // Check for common passwords
        info.is_common = Self::is_common_password(password);

        // Generate feedback
        info.feedback = Self::generate_password_feedback(&info, password);

        info
    }

    /// Calculate password entropy
    fn calculate_entropy(password: &str) -> f64 {
        if password.is_empty() {
            return 0.0;
        }

        let mut charset_size = 0;
        if password.chars().any(|c| c.is_ascii_lowercase()) { charset_size += 26; }
        if password.chars().any(|c| c.is_ascii_uppercase()) { charset_size += 26; }
        if password.chars().any(|c| c.is_ascii_digit()) { charset_size += 10; }
        if password.chars().any(|c| !c.is_alphanumeric()) { charset_size += 32; }

        if charset_size == 0 {
            return 0.0;
        }

        (password.len() as f64) * (charset_size as f64).log2()
    }

    /// Calculate comprehensive strength score
    fn calculate_strength_score(info: &PasswordStrengthInfo, password: &str) -> u32 {
        let mut score = 0u32;

        // Length scoring
        score += match info.length {
            0..=7 => 0,
            8..=11 => 20,
            12..=15 => 35,
            _ => 50,
        };

        // Character variety
        let variety_count = [info.has_lowercase, info.has_uppercase, info.has_numbers, info.has_symbols]
            .iter().filter(|&&x| x).count();
        score += variety_count as u32 * 10;

        // Entropy bonus
        if info.entropy >= 60.0 { score += 20; }
        else if info.entropy >= 40.0 { score += 10; }

        // Penalties
        if info.is_common { score = score.saturating_sub(30); }
        if info.is_compromised { score = score.saturating_sub(50); }

        // Pattern detection penalties
        if Self::has_sequential_chars(password) { score = score.saturating_sub(10); }
        if Self::has_repeated_chars(password) { score = score.saturating_sub(15); }

        score.min(100)
    }

    /// Check for sequential characters
    fn has_sequential_chars(password: &str) -> bool {
        let chars: Vec<char> = password.chars().collect();
        chars.windows(3).any(|w| {
            w[0] as u8 + 1 == w[1] as u8 && w[1] as u8 + 1 == w[2] as u8
        })
    }

    /// Check for repeated characters
    fn has_repeated_chars(password: &str) -> bool {
        let chars: Vec<char> = password.chars().collect();
        chars.windows(3).any(|w| w[0] == w[1] && w[1] == w[2])
    }

    /// Check if password is commonly used
    fn is_common_password(password: &str) -> bool {
        // Basic common password list - in production, use comprehensive lists
        let common_passwords = [
            "password", "123456", "password123", "admin", "qwerty",
            "letmein", "welcome", "monkey", "dragon", "master",
            "password1", "123456789", "1234567890"
        ];
        
        common_passwords.iter().any(|&common| 
            password.to_lowercase() == common.to_lowercase()
        )
    }

    /// Generate improvement feedback
    fn generate_password_feedback(info: &PasswordStrengthInfo, password: &str) -> Vec<String> {
        let mut feedback = Vec::new();

        if info.length < 12 {
            feedback.push("Use at least 12 characters for better security".to_string());
        }

        if !info.has_lowercase || !info.has_uppercase {
            feedback.push("Include both uppercase and lowercase letters".to_string());
        }

        if !info.has_numbers {
            feedback.push("Add numbers to increase complexity".to_string());
        }

        if !info.has_symbols {
            feedback.push("Include special characters (!@#$%^&*)".to_string());
        }

        if info.is_common {
            feedback.push("Avoid common passwords - use a unique combination".to_string());
        }

        if info.is_compromised {
            feedback.push("⚠️ This password appears in data breaches - change immediately!".to_string());
        }

        if Self::has_sequential_chars(password) {
            feedback.push("Avoid sequential characters (abc, 123)".to_string());
        }

        if Self::has_repeated_chars(password) {
            feedback.push("Avoid repeated characters (aaa, 111)".to_string());
        }

        feedback
    }

    /// Update overall risk level based on various factors
    pub fn update_risk_level(&mut self) {
        let mut risk_score = 0u32;

        // Password strength impact
        risk_score += match self.password_strength.score {
            0..=30 => 40,   // Weak password = high risk
            31..=60 => 20,  // Fair password = medium risk
            61..=80 => 10,  // Good password = low risk
            _ => 0,         // Strong password = minimal risk
        };

        // Breach status impact
        if self.security_info.breach_check_status.is_compromised {
            risk_score += 50;
        }

        // Age of password
        let password_age = Utc::now().signed_duration_since(self.modified_at).num_days();
        if password_age > 365 { risk_score += 20; }
        else if password_age > 180 { risk_score += 10; }

        // Two-factor authentication
        if let Some(false) = self.security_info.two_factor_enabled {
            risk_score += 15;
        }

        // Determine risk level
        self.security_info.risk_level = match risk_score {
            0..=20 => RiskLevel::Low,
            21..=40 => RiskLevel::Medium,
            41..=70 => RiskLevel::High,
            _ => RiskLevel::Critical,
        };
    }

    /// Mark entry as accessed (for usage tracking)
    pub fn mark_accessed(&mut self) {
        self.last_accessed = Some(Utc::now());
    }

    /// Add a custom field
    pub fn add_custom_field(&mut self, key: String, value: String) {
        self.custom_fields.insert(key, value);
        self.modified_at = Utc::now();
    }

    /// Add a tag
    pub fn add_tag(&mut self, tag: String) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
            self.modified_at = Utc::now();
        }
    }

    /// Set expiration policy
    pub fn set_expiration(&mut self, days_from_now: u32, warning_days: u32) {
        self.expiration = Some(PasswordExpiration {
            expires_at: Utc::now() + Duration::days(days_from_now as i64),
            warning_days,
            auto_remind: true,
            policy_source: "User Set".to_string(),
        });
        self.modified_at = Utc::now();
    }

    /// Check if password is expired or near expiration
    pub fn check_expiration_status(&self) -> ExpirationStatus {
        if let Some(ref expiration) = self.expiration {
            let now = Utc::now();
            let warning_time = expiration.expires_at - Duration::days(expiration.warning_days as i64);
            
            if now >= expiration.expires_at {
                ExpirationStatus::Expired
            } else if now >= warning_time {
                ExpirationStatus::Warning(expiration.expires_at.signed_duration_since(now).num_days() as u32)
            } else {
                ExpirationStatus::Valid
            }
        } else {
            ExpirationStatus::NoExpiration
        }
    }

    /// Soft delete the entry
    pub fn soft_delete(&mut self) {
        self.is_deleted = true;
        self.deleted_at = Some(Utc::now());
        self.modified_at = Utc::now();
    }

    /// Restore soft deleted entry
    pub fn restore(&mut self) {
        self.is_deleted = false;
        self.deleted_at = None;
        self.modified_at = Utc::now();
    }

    /// Get entry age in days
    pub fn get_age_days(&self) -> i64 {
        Utc::now().signed_duration_since(self.created_at).num_days()
    }

    /// Get password age in days
    pub fn get_password_age_days(&self) -> i64 {
        Utc::now().signed_duration_since(self.modified_at).num_days()
    }

    /// Check if password was reused from history
    pub fn is_password_reused(&self, password: &str) -> bool {
        let new_hash = self.hash_password(password);
        self.password_history.iter().any(|entry| entry.password_hash == new_hash)
    }

    /// Generate a simple hash for password comparison
    fn hash_password(&self, password: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        password.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Export entry data (with optional password masking)
    pub fn to_export_format(&self, include_password: bool) -> ExportEntry {
        ExportEntry {
            service: self.service.clone(),
            username: self.username.clone(),
            password: if include_password { 
                self.password.clone() 
            } else { 
                "********".to_string() 
            },
            url: self.url.clone(),
            notes: self.notes.clone(),
            category: self.category.clone(),
            tags: self.tags.clone(),
            created_at: self.created_at,
            modified_at: self.modified_at,
        }
    }
}

/// Password expiration status
#[derive(Debug, PartialEq)]
pub enum ExpirationStatus {
    Valid,
    Warning(u32),  // Days until expiration
    Expired,
    NoExpiration,
}

/// Simplified entry format for export
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ExportEntry {
    pub service: String,
    pub username: String,
    pub password: String,
    pub url: Option<String>,
    pub notes: Option<String>,
    pub category: EntryCategory,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
}

// Helper functions for serde defaults
fn generate_entry_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

fn current_timestamp() -> DateTime<Utc> {
    Utc::now()
}

fn default_category() -> EntryCategory {
    EntryCategory::Personal
}

impl Default for EntryCategory {
    fn default() -> Self {
        EntryCategory::Personal
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entry_creation() {
        let entry = Entry::new(
            "Gmail".to_string(),
            "user@example.com".to_string(),
            "MySecurePassword123!".to_string()
        );
        
        assert_eq!(entry.service, "Gmail");
        assert_eq!(entry.username, "user@example.com");
        assert!(!entry.id.is_empty());
        assert!(entry.password_strength.score > 0);
    }

    #[test]
    fn test_password_strength_assessment() {
        let weak_info = Entry::assess_password_strength("123456");
        assert!(weak_info.score <= 30);
        assert_eq!(weak_info.level, "Weak");

        let strong_info = Entry::assess_password_strength("MyVerySecureP@ssw0rd2024!");
        assert!(strong_info.score >= 80);
        assert_eq!(strong_info.level, "Strong");
    }

    #[test]
    fn test_password_update_with_history() {
        let mut entry = Entry::new("Test".to_string(), "user".to_string(), "old_password".to_string());
        
        entry.update_password("new_password".to_string(), "Manual Change");
        
        assert_eq!(entry.password, "new_password");
        assert_eq!(entry.password_history.len(), 1);
        assert_eq!(entry.password_history[0].changed_reason, "Manual Change");
    }

    #[test]
    fn test_expiration_checking() {
        let mut entry = Entry::new("Test".to_string(), "user".to_string(), "password".to_string());
        
        entry.set_expiration(1, 0); // Expires in 1 day, no warning
        
        match entry.check_expiration_status() {
            ExpirationStatus::Valid => assert!(true),
            _ => assert!(false, "Should be valid for 1 day"),
        }
    }

    #[test]
    fn test_soft_delete_and_restore() {
        let mut entry = Entry::new("Test".to_string(), "user".to_string(), "password".to_string());
        
        assert!(!entry.is_deleted);
        
        entry.soft_delete();
        assert!(entry.is_deleted);
        assert!(entry.deleted_at.is_some());
        
        entry.restore();
        assert!(!entry.is_deleted);
        assert!(entry.deleted_at.is_none());
    }
}
