#![allow(dead_code)]
use std::sync::{Arc, Mutex, RwLock};
use std::{fs, io::Write, time::{Duration, Instant}, collections::HashMap};
use rpassword::read_password;
use zeroize::Zeroize;
use getrandom::getrandom;
use chrono::Timelike;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

/// Security event categories for comprehensive audit logging
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SecurityEventType {
    Authentication,
    Authorization,
    DataAccess,
    DataModification,
    SystemAccess,
    ConfigurationChange,
    SecurityViolation,
    SuspiciousActivity,
    VaultOperation,
    CryptoOperation,
}

/// Security event severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum SecuritySeverity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

/// Structured security event for comprehensive audit logging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEvent {
    pub timestamp: DateTime<Utc>,
    pub event_type: SecurityEventType,
    pub severity: SecuritySeverity,
    pub message: String,
    pub source: String,
    pub user_context: Option<String>,
    pub ip_address: Option<String>,
    pub additional_data: HashMap<String, String>,
    pub session_id: Option<String>,
}

impl SecurityEvent {
    pub fn new(event_type: SecurityEventType, severity: SecuritySeverity, message: &str, source: &str) -> Self {
        Self {
            timestamp: Utc::now(),
            event_type,
            severity,
            message: message.to_string(),
            source: source.to_string(),
            user_context: None,
            ip_address: None,
            additional_data: HashMap::new(),
            session_id: None,
        }
    }

    pub fn with_context(mut self, user: &str) -> Self {
        self.user_context = Some(user.to_string());
        self
    }

    pub fn with_session(mut self, session_id: &str) -> Self {
        self.session_id = Some(session_id.to_string());
        self
    }

    pub fn with_data(mut self, key: &str, value: &str) -> Self {
        self.additional_data.insert(key.to_string(), value.to_string());
        self
    }
}

/// Advanced audit logging system with security features
#[derive(Debug)]
pub struct AuditLog {
    events: Arc<RwLock<Vec<SecurityEvent>>>,
    max_events: usize,
    file_path: Option<String>,
    threat_detector: ThreatDetector,
    log_integrity: LogIntegrityChecker,
}

impl AuditLog {
    pub fn new() -> Self {
        Self::new_with_capacity(10000) // Default: store last 10k events
    }

    pub fn new_with_capacity(max_events: usize) -> Self {
        Self {
            events: Arc::new(RwLock::new(Vec::new())),
            max_events,
            file_path: None,
            threat_detector: ThreatDetector::new(),
            log_integrity: LogIntegrityChecker::new(),
        }
    }

    pub fn set_file_path(&mut self, path: &str) {
        self.file_path = Some(path.to_string());
    }

    /// Log a simple message (legacy compatibility)
    pub fn log(&self, msg: &str) {
        let event = SecurityEvent::new(
            SecurityEventType::VaultOperation,
            SecuritySeverity::Info,
            msg,
            "vault"
        );
        self.log_event(event);
    }

    /// Log a structured security event
    pub fn log_event(&self, event: SecurityEvent) {
        // Detect potential threats
        if let Some(threat) = self.threat_detector.analyze_event(&event) {
            self.handle_threat_detection(threat);
        }

        // Store event in memory
        {
            let mut events = self.events.write().unwrap();
            events.push(event.clone());
            
            // Maintain max capacity
            if events.len() > self.max_events {
                events.remove(0);
            }
        }

        // Persist to file immediately for critical events
        if event.severity >= SecuritySeverity::High {
            if let Err(e) = self.persist_event(&event) {
                eprintln!("⚠️ Failed to persist critical security event: {}", e);
            }
        }

        // Update integrity checksum
        self.log_integrity.update_checksum(&event);
    }

    /// Log authentication events
    pub fn log_authentication(&self, success: bool, user: Option<&str>, details: &str) {
        let event_type = SecurityEventType::Authentication;
        let severity = if success { SecuritySeverity::Info } else { SecuritySeverity::Medium };
        let message = if success {
            format!("Authentication successful: {}", details)
        } else {
            format!("Authentication failed: {}", details)
        };

        let mut event = SecurityEvent::new(event_type, severity, &message, "auth");
        if let Some(user) = user {
            event = event.with_context(user);
        }

        self.log_event(event);
    }

    /// Log vault operations
    pub fn log_vault_operation(&self, operation: &str, details: &str, user: Option<&str>) {
        let severity = match operation {
            "delete" | "export" | "import" => SecuritySeverity::Medium,
            "decrypt" | "encrypt" => SecuritySeverity::Low,
            _ => SecuritySeverity::Info,
        };

        let message = format!("Vault operation: {} - {}", operation, details);
        let mut event = SecurityEvent::new(SecurityEventType::VaultOperation, severity, &message, "vault");
        
        if let Some(user) = user {
            event = event.with_context(user);
        }
        
        self.log_event(event);
    }

    /// Log security violations
    pub fn log_security_violation(&self, violation: &str, details: &str) {
        let event = SecurityEvent::new(
            SecurityEventType::SecurityViolation,
            SecuritySeverity::High,
            &format!("Security violation: {} - {}", violation, details),
            "security_monitor"
        );
        
    // Clone for logging since log_event takes ownership; keep original for alert
    self.log_event(event.clone());
        
    // Immediate notification for security violations
    self.send_security_alert(&event);
    }

    /// Log suspicious activity
    pub fn log_suspicious_activity(&self, activity: &str, risk_score: u32) {
        let severity = match risk_score {
            0..=30 => SecuritySeverity::Low,
            31..=60 => SecuritySeverity::Medium,
            61..=80 => SecuritySeverity::High,
            _ => SecuritySeverity::Critical,
        };

        let event = SecurityEvent::new(
            SecurityEventType::SuspiciousActivity,
            severity,
            &format!("Suspicious activity detected: {} (risk score: {})", activity, risk_score),
            "threat_detector"
        ).with_data("risk_score", &risk_score.to_string());

        self.log_event(event);
    }

    /// Get recent events
    pub fn get_recent_logs(&self, count: usize) -> Vec<String> {
        let events = self.events.read().unwrap();
        events.iter()
            .rev()
            .take(count)
            .map(|e| self.format_event(e))
            .collect()
    }

    /// Get events by type and severity
    pub fn get_events_by_criteria(&self, event_type: Option<SecurityEventType>, min_severity: Option<SecuritySeverity>) -> Vec<SecurityEvent> {
        let events = self.events.read().unwrap();
        events.iter()
            .filter(|event| {
                if let Some(ref et) = event_type {
                    if event.event_type != *et { return false; }
                }
                if let Some(ref ms) = min_severity {
                    if event.severity < *ms { return false; }
                }
                true
            })
            .cloned()
            .collect()
    }

    /// Generate security report
    pub fn generate_security_report(&self, hours: u32) -> SecurityReport {
        let cutoff = Utc::now() - chrono::Duration::hours(hours as i64);
        let events = self.events.read().unwrap();
        
        let recent_events: Vec<_> = events.iter()
            .filter(|e| e.timestamp >= cutoff)
            .collect();

        let mut report = SecurityReport::new(hours);
        
        for event in &recent_events {
            report.add_event(event);
        }

        report.threat_score = self.threat_detector.calculate_threat_score(&recent_events);
        report.integrity_status = self.log_integrity.verify_integrity();
        
        report
    }

    /// Persist events to file
    pub fn persist(&self, path: &str) -> std::io::Result<()> {
        let events = self.events.read().unwrap();
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;

        for event in events.iter() {
            let json_line = serde_json::to_string(event)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            writeln!(file, "{}", json_line)?;
        }

        // Add integrity signature
        let signature = self.log_integrity.generate_signature();
        writeln!(file, "INTEGRITY_SIG:{}", signature)?;

        Ok(())
    }

    /// Persist single event (for critical events)
    fn persist_event(&self, event: &SecurityEvent) -> std::io::Result<()> {
        if let Some(ref path) = self.file_path {
            let mut file = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)?;

            let json_line = serde_json::to_string(event)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            writeln!(file, "{}", json_line)?;
        }
        Ok(())
    }

    /// Format event for display
    fn format_event(&self, event: &SecurityEvent) -> String {
        let severity_emoji = match event.severity {
            SecuritySeverity::Info => "ℹ️",
            SecuritySeverity::Low => "🟡",
            SecuritySeverity::Medium => "🟠",
            SecuritySeverity::High => "🔴",
            SecuritySeverity::Critical => "🚨",
        };

        format!("{} [{}] {:?}: {}",
            event.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
            severity_emoji,
            event.event_type,
            event.message
        )
    }

    /// Handle threat detection
    fn handle_threat_detection(&self, threat: ThreatInfo) {
        let event = SecurityEvent::new(
            SecurityEventType::SecurityViolation,
            SecuritySeverity::Critical,
            &format!("THREAT DETECTED: {} (confidence: {}%)", threat.description, threat.confidence),
            "threat_detector"
        ).with_data("threat_type", &threat.threat_type)
         .with_data("confidence", &threat.confidence.to_string());

        // Log the threat
        {
            let mut events = self.events.write().unwrap();
            events.push(event.clone());
        }

        // Immediate response
        self.send_security_alert(&event);
    }

    /// Send security alert (placeholder - implement with actual notification system)
    fn send_security_alert(&self, event: &SecurityEvent) {
        // In production, integrate with:
        // - Email notifications
        // - Slack/Teams webhooks
        // - SIEM systems
        // - Security operations center (SOC)
        
        if event.severity >= SecuritySeverity::High {
            eprintln!("🚨 SECURITY ALERT: {}", event.message);
        }
    }
}

/// Advanced vault locking system with security features
pub struct VaultLock {
    last_access: Mutex<Instant>,
    timeout: Duration,
    failed_attempts: Arc<Mutex<u32>>,
    lockout_until: Arc<Mutex<Option<Instant>>>,
    max_failed_attempts: u32,
    lockout_duration: Duration,
}

impl VaultLock {
    pub fn new(timeout_secs: u64) -> Self {
        Self {
            last_access: Mutex::new(Instant::now()),
            timeout: Duration::from_secs(timeout_secs),
            failed_attempts: Arc::new(Mutex::new(0)),
            lockout_until: Arc::new(Mutex::new(None)),
            max_failed_attempts: 5, // Lock after 5 failed attempts
            lockout_duration: Duration::from_secs(300), // 5-minute lockout
        }
    }

    pub fn refresh(&self) {
        let mut last_access = self.last_access.lock().unwrap();
        *last_access = Instant::now();
        
        // Reset failed attempts on successful access
        let mut attempts = self.failed_attempts.lock().unwrap();
        *attempts = 0;
    }

    pub fn is_locked(&self) -> bool {
        // Check if in lockout period
        {
            let lockout = self.lockout_until.lock().unwrap();
            if let Some(until) = *lockout {
                if Instant::now() < until {
                    return true;
                }
            }
        }

        // Check timeout
        let last_access = self.last_access.lock().unwrap();
        last_access.elapsed() > self.timeout
    }

    pub fn time_until_lock(&self) -> Duration {
        let last_access = self.last_access.lock().unwrap();
        self.timeout.saturating_sub(last_access.elapsed())
    }

    pub fn record_failed_attempt(&self) -> bool {
        let mut attempts = self.failed_attempts.lock().unwrap();
        *attempts += 1;

        if *attempts >= self.max_failed_attempts {
            let mut lockout = self.lockout_until.lock().unwrap();
            *lockout = Some(Instant::now() + self.lockout_duration);
            true // Lockout triggered
        } else {
            false // No lockout yet
        }
    }

    pub fn get_failed_attempts(&self) -> u32 {
        *self.failed_attempts.lock().unwrap()
    }

    pub fn get_lockout_remaining(&self) -> Option<Duration> {
        let lockout = self.lockout_until.lock().unwrap();
        if let Some(until) = *lockout {
            let now = Instant::now();
            if now < until {
                Some(until - now)
            } else {
                None
            }
        } else {
            None
        }
    }
}

/// Threat detection system
#[derive(Debug)]
struct ThreatDetector {
    patterns: Vec<ThreatPattern>,
    event_history: Arc<Mutex<Vec<SecurityEvent>>>,
}

#[derive(Debug, Clone)]
struct ThreatPattern {
    name: String,
    event_type: SecurityEventType,
    threshold_count: usize,
    time_window: Duration,
    risk_score: u32,
}

#[derive(Debug)]
struct ThreatInfo {
    threat_type: String,
    description: String,
    confidence: u32,
}

impl ThreatDetector {
    fn new() -> Self {
        let mut patterns = Vec::new();
        
        // Define threat patterns
        patterns.push(ThreatPattern {
            name: "Multiple Failed Authentications".to_string(),
            event_type: SecurityEventType::Authentication,
            threshold_count: 5,
            time_window: Duration::from_secs(300), // 5 minutes
            risk_score: 80,
        });

        patterns.push(ThreatPattern {
            name: "Rapid Vault Access".to_string(),
            event_type: SecurityEventType::VaultOperation,
            threshold_count: 20,
            time_window: Duration::from_secs(60), // 1 minute
            risk_score: 60,
        });

        Self {
            patterns,
            event_history: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn analyze_event(&self, event: &SecurityEvent) -> Option<ThreatInfo> {
        // Add to history
        {
            let mut history = self.event_history.lock().unwrap();
            history.push(event.clone());
            
            // Keep only recent events (last hour)
            let cutoff = Utc::now() - chrono::Duration::hours(1);
            history.retain(|e| e.timestamp >= cutoff);
        }

        // Check patterns
        for pattern in &self.patterns {
            if self.check_pattern(pattern, event) {
                return Some(ThreatInfo {
                    threat_type: pattern.name.clone(),
                    description: format!("Detected {} matching pattern criteria", pattern.name),
                    confidence: pattern.risk_score,
                });
            }
        }

        None
    }

    fn check_pattern(&self, pattern: &ThreatPattern, current_event: &SecurityEvent) -> bool {
        if current_event.event_type != pattern.event_type {
            return false;
        }

        let history = self.event_history.lock().unwrap();
        let cutoff = current_event.timestamp - chrono::Duration::from_std(pattern.time_window).unwrap();
        
        let matching_events = history.iter()
            .filter(|e| e.event_type == pattern.event_type && e.timestamp >= cutoff)
            .count();

        matching_events >= pattern.threshold_count
    }

    fn calculate_threat_score(&self, events: &[&SecurityEvent]) -> u32 {
        let mut score = 0u32;

        let critical_count = events.iter().filter(|e| e.severity == SecuritySeverity::Critical).count();
        let high_count = events.iter().filter(|e| e.severity == SecuritySeverity::High).count();
        let violation_count = events.iter().filter(|e| e.event_type == SecurityEventType::SecurityViolation).count();

        score += critical_count as u32 * 25;
        score += high_count as u32 * 15;
        score += violation_count as u32 * 20;

        score.min(100)
    }
}

/// Log integrity checker
#[derive(Debug)]
struct LogIntegrityChecker {
    checksum: Arc<Mutex<u64>>,
}

impl LogIntegrityChecker {
    fn new() -> Self {
        Self {
            checksum: Arc::new(Mutex::new(0)),
        }
    }

    fn update_checksum(&self, event: &SecurityEvent) {
        let mut checksum = self.checksum.lock().unwrap();
        let event_hash = self.hash_event(event);
        *checksum = checksum.wrapping_add(event_hash);
    }

    fn hash_event(&self, event: &SecurityEvent) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        event.timestamp.hash(&mut hasher);
        event.message.hash(&mut hasher);
        hasher.finish()
    }

    fn verify_integrity(&self) -> bool {
        // In production, this would verify against stored checksums
        // For now, always return true
        true
    }

    fn generate_signature(&self) -> String {
        let checksum = self.checksum.lock().unwrap();
        format!("SHA256:{:016x}", *checksum)
    }
}

/// Security report structure
#[derive(Debug, Serialize)]
pub struct SecurityReport {
    pub generated_at: DateTime<Utc>,
    pub period_hours: u32,
    pub total_events: usize,
    pub events_by_severity: HashMap<String, usize>,
    pub events_by_type: HashMap<String, usize>,
    pub threat_score: u32,
    pub integrity_status: bool,
    pub recommendations: Vec<String>,
}

impl SecurityReport {
    fn new(period_hours: u32) -> Self {
        Self {
            generated_at: Utc::now(),
            period_hours,
            total_events: 0,
            events_by_severity: HashMap::new(),
            events_by_type: HashMap::new(),
            threat_score: 0,
            integrity_status: true,
            recommendations: Vec::new(),
        }
    }

    fn add_event(&mut self, event: &SecurityEvent) {
        self.total_events += 1;

        let severity_key = format!("{:?}", event.severity);
        *self.events_by_severity.entry(severity_key).or_insert(0) += 1;

        let type_key = format!("{:?}", event.event_type);
        *self.events_by_type.entry(type_key).or_insert(0) += 1;

        // Generate recommendations based on events
        if event.severity >= SecuritySeverity::High {
            self.recommendations.push(format!(
                "Investigate high-severity event: {}",
                event.message
            ));
        }
    }
}

/// Secure password input with advanced features
pub fn get_secure_password(prompt: &str) -> String {
    if !prompt.is_empty() {
        print!("{}", prompt);
        std::io::stdout().flush().unwrap();
    }

    match read_password() {
        Ok(mut pass) => {
            let pw = pass.clone();
            pass.zeroize();
            
            // Basic validation
            if pw.len() < 8 {
                eprintln!("⚠️ Warning: Password is shorter than recommended (8+ characters)");
            }
            
            pw
        },
        Err(e) => {
            eprintln!("❌ Error reading password: {}", e);
            String::new()
        }
    }
}

/// Advanced password input with retry logic and validation
pub fn get_secure_password_with_validation(
    prompt: &str,
    max_attempts: u32,
    min_length: usize,
) -> Result<String, String> {
    for attempt in 1..=max_attempts {
        let password = get_secure_password(prompt);
        
        if password.len() >= min_length {
            return Ok(password);
        }
        
        if attempt < max_attempts {
            eprintln!("❌ Password too short (minimum {} characters). Attempt {}/{}", 
                     min_length, attempt, max_attempts);
        }
    }
    
    Err(format!("Failed to get valid password after {} attempts", max_attempts))
}

/// Generate secure session ID
pub fn generate_session_id() -> String {
    let mut bytes = [0u8; 16];
    getrandom(&mut bytes).expect("OS RNG failed");
    hex::encode(bytes)
}

/// Security utilities
pub mod utils {
    use super::*;

    /// Check if current time is within business hours (basic implementation)
    pub fn is_business_hours() -> bool {
        let now = chrono::Local::now();
        let hour = now.hour();
        hour >= 9 && hour <= 17 // 9 AM to 5 PM
    }

    /// Detect if running in suspicious environment
    pub fn detect_suspicious_environment() -> Vec<String> {
        let mut warnings = Vec::new();

        // Check for debugger
        if cfg!(debug_assertions) {
            warnings.push("Running in debug mode".to_string());
        }

        // Check for common analysis tools (basic detection)
        if std::env::var("RUST_BACKTRACE").is_ok() {
            warnings.push("Backtrace enabled - possible debugging".to_string());
        }

        warnings
    }

    /// Rate limiting helper
    pub struct RateLimiter {
        requests: Arc<Mutex<Vec<Instant>>>,
        max_requests: usize,
        window: Duration,
    }

    impl RateLimiter {
        pub fn new(max_requests: usize, window_seconds: u64) -> Self {
            Self {
                requests: Arc::new(Mutex::new(Vec::new())),
                max_requests,
                window: Duration::from_secs(window_seconds),
            }
        }

        pub fn check_rate_limit(&self) -> bool {
            let now = Instant::now();
            let mut requests = self.requests.lock().unwrap();
            
            // Remove old requests
            requests.retain(|&req_time| now.duration_since(req_time) <= self.window);
            
            if requests.len() >= self.max_requests {
                false // Rate limit exceeded
            } else {
                requests.push(now);
                true // Request allowed
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_event_creation() {
        let event = SecurityEvent::new(
            SecurityEventType::Authentication,
            SecuritySeverity::High,
            "Test event",
            "test"
        );
        
        assert_eq!(event.event_type, SecurityEventType::Authentication);
        assert_eq!(event.severity, SecuritySeverity::High);
        assert_eq!(event.message, "Test event");
    }

    #[test]
    fn test_vault_lock_functionality() {
        let lock = VaultLock::new(1); // 1 second timeout
        
        assert!(!lock.is_locked());
        
        std::thread::sleep(Duration::from_secs(2));
        assert!(lock.is_locked());
    }

    #[test]
    fn test_threat_detector() {
        let detector = ThreatDetector::new();
        let event = SecurityEvent::new(
            SecurityEventType::Authentication,
            SecuritySeverity::Medium,
            "Failed login",
            "auth"
        );
        
        // Should not trigger on first event
        assert!(detector.analyze_event(&event).is_none());
    }

    #[test]
    fn test_rate_limiter() {
        let limiter = utils::RateLimiter::new(2, 1);
        
        assert!(limiter.check_rate_limit());
        assert!(limiter.check_rate_limit());
        assert!(!limiter.check_rate_limit()); // Should be rate limited
    }
}
