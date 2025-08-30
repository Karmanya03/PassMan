use axum::{
    extract::{Json, Path, State},
    http::{HeaderMap, StatusCode},
    middleware,
    response::Json as ResponseJson,
    routing::{delete, get, post, put},
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use uuid::Uuid;

mod auth;
mod crypto;
mod db;
mod entry;
mod security;
mod vault;

use crate::auth::{Claims, JwtManager};
use crate::entry::{Entry, EntryCategory};
use crate::vault::Vault;

#[derive(Clone)]
pub struct AppState {
    vaults: Arc<Mutex<HashMap<String, Vault>>>, // user_id -> vault
    jwt_manager: JwtManager,
    db: Arc<Mutex<db::Database>>,
}

#[derive(Serialize, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub master_password: String,
}

#[derive(Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub master_password: String,
}

#[derive(Serialize, Deserialize)]
pub struct AuthResponse {
    pub token: String,
    pub user_id: String,
    pub username: String,
}

#[derive(Serialize, Deserialize)]
pub struct AddEntryRequest {
    pub service: String,
    pub username: String,
    pub password: String,
    pub url: Option<String>,
    pub notes: Option<String>,
    pub category: EntryCategory,
    pub tags: Vec<String>,
    pub two_factor_enabled: Option<bool>,
}

#[derive(Serialize, Deserialize)]
pub struct UpdateEntryRequest {
    pub service: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub url: Option<String>,
    pub notes: Option<String>,
    pub category: Option<EntryCategory>,
    pub tags: Option<Vec<String>>,
    pub is_favorite: Option<bool>,
    pub two_factor_enabled: Option<bool>,
}

#[derive(Serialize, Deserialize)]
pub struct GeneratePasswordRequest {
    pub length: usize,
    pub symbols: bool,
    pub count: usize,
}

#[derive(Serialize, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub case_sensitive: Option<bool>,
}

#[derive(Serialize, Deserialize)]
pub struct VaultStats {
    pub total_entries: usize,
    pub categories_count: HashMap<String, usize>,
    pub weak_passwords: usize,
    pub duplicate_passwords: usize,
    pub old_passwords: usize,
    pub breached_passwords: usize,
    pub two_factor_enabled: usize,
    pub last_backup: String,
    pub vault_size: String,
    pub security_score: u8,
}

#[derive(Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub message: Option<String>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: None,
            error: None,
        }
    }

    pub fn success_with_message(data: T, message: String) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: Some(message),
            error: None,
        }
    }

    pub fn error(error: String) -> Self {
        Self {
            success: false,
            data: None,
            message: None,
            error: Some(error),
        }
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let db = Arc::new(Mutex::new(db::Database::new().expect("Failed to initialize database")));
    let jwt_manager = JwtManager::new();

    let app_state = AppState {
        vaults: Arc::new(Mutex::new(HashMap::new())),
        jwt_manager,
        db,
    };

    let app = Router::new()
        // Authentication routes
        .route("/api/auth/register", post(register))
        .route("/api/auth/login", post(login))
        .route("/api/auth/logout", post(logout))
        .route("/api/auth/verify", get(verify_token))
        
        // Vault routes (protected)
        .route("/api/vault/unlock", post(unlock_vault))
        .route("/api/vault/lock", post(lock_vault))
        .route("/api/vault/status", get(vault_status))
        .route("/api/vault/stats", get(vault_stats))
        
        // Entry routes (protected)
        .route("/api/entries", get(get_entries))
        .route("/api/entries", post(add_entry))
        .route("/api/entries/:id", get(get_entry))
        .route("/api/entries/:id", put(update_entry))
        .route("/api/entries/:id", delete(delete_entry))
        .route("/api/entries/search", post(search_entries))
        
        // Utility routes
        .route("/api/generate-password", post(generate_password))
        .route("/api/check-password-strength", post(check_password_strength))
        
        // Audit routes
        .route("/api/audit/logs", get(get_audit_logs))
        
        // Export/Import routes
        .route("/api/export", post(export_vault))
        .route("/api/import", post(import_vault))
        
        // Health check
        .route("/api/health", get(health_check))
        
        // Serve static files (frontend)
        .nest_service("/", ServeDir::new("frontend/out"))
        
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(middleware::from_fn_with_state(app_state.clone(), auth::auth_middleware))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    println!("🚀 PassMan Server running on http://localhost:8080");
    
    axum::serve(listener, app).await.unwrap();
}

// Authentication handlers
async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<ResponseJson<ApiResponse<AuthResponse>>, StatusCode> {
    let mut db = state.db.lock().unwrap();
    
    // Validate input
    if payload.username.is_empty() || payload.email.is_empty() || payload.master_password.len() < 8 {
        return Ok(ResponseJson(ApiResponse::error(
            "Invalid input: username and email required, password must be 8+ characters".to_string()
        )));
    }

    // Check if user exists
    if db.user_exists(&payload.username).unwrap_or(false) {
        return Ok(ResponseJson(ApiResponse::error(
            "Username already exists".to_string()
        )));
    }

    // Create user
    let user_id = Uuid::new_v4().to_string();
    match db.create_user(&user_id, &payload.username, &payload.email, &payload.master_password) {
        Ok(_) => {
            // Generate JWT token
            let token = state.jwt_manager.generate_token(&user_id, &payload.username)?;
            
            // Create empty vault for user
            let vault = Vault::new(900); // 15 minute timeout
            state.vaults.lock().unwrap().insert(user_id.clone(), vault);
            
            let response = AuthResponse {
                token,
                user_id,
                username: payload.username,
            };
            
            Ok(ResponseJson(ApiResponse::success_with_message(
                response,
                "Registration successful".to_string()
            )))
        }
        Err(e) => Ok(ResponseJson(ApiResponse::error(format!("Registration failed: {}", e)))),
    }
}

async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<ResponseJson<ApiResponse<AuthResponse>>, StatusCode> {
    let db = state.db.lock().unwrap();
    
    match db.verify_user(&payload.username, &payload.master_password) {
        Ok(Some(user_id)) => {
            // Generate JWT token
            let token = state.jwt_manager.generate_token(&user_id, &payload.username)?;
            
            // Load user's vault
            match db.load_vault(&user_id, &payload.master_password) {
                Ok(vault) => {
                    state.vaults.lock().unwrap().insert(user_id.clone(), vault);
                }
                Err(_) => {
                    // Create new vault if none exists
                    let vault = Vault::new(900);
                    state.vaults.lock().unwrap().insert(user_id.clone(), vault);
                }
            }
            
            let response = AuthResponse {
                token,
                user_id,
                username: payload.username,
            };
            
            Ok(ResponseJson(ApiResponse::success_with_message(
                response,
                "Login successful".to_string()
            )))
        }
        Ok(None) => Ok(ResponseJson(ApiResponse::error(
            "Invalid credentials".to_string()
        ))),
        Err(e) => Ok(ResponseJson(ApiResponse::error(format!("Login failed: {}", e)))),
    }
}

async fn logout(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<ResponseJson<ApiResponse<String>>, StatusCode> {
    if let Some(claims) = auth::extract_claims(&headers, &state.jwt_manager) {
        // Remove vault from memory
        state.vaults.lock().unwrap().remove(&claims.user_id);
        
        Ok(ResponseJson(ApiResponse::success_with_message(
            "Logged out".to_string(),
            "Logout successful".to_string()
        )))
    } else {
        Ok(ResponseJson(ApiResponse::error("Not authenticated".to_string())))
    }
}

async fn verify_token(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<ResponseJson<ApiResponse<Claims>>, StatusCode> {
    if let Some(claims) = auth::extract_claims(&headers, &state.jwt_manager) {
        Ok(ResponseJson(ApiResponse::success(claims)))
    } else {
        Ok(ResponseJson(ApiResponse::error("Invalid token".to_string())))
    }
}

// Vault handlers
async fn unlock_vault(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<ResponseJson<ApiResponse<String>>, StatusCode> {
    if let Some(claims) = auth::extract_claims(&headers, &state.jwt_manager) {
        let vaults = state.vaults.lock().unwrap();
        if vaults.contains_key(&claims.user_id) {
            Ok(ResponseJson(ApiResponse::success_with_message(
                "Vault unlocked".to_string(),
                "Vault is ready for use".to_string()
            )))
        } else {
            Ok(ResponseJson(ApiResponse::error("Vault not found".to_string())))
        }
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

async fn lock_vault(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<ResponseJson<ApiResponse<String>>, StatusCode> {
    if let Some(claims) = auth::extract_claims(&headers, &state.jwt_manager) {
        state.vaults.lock().unwrap().remove(&claims.user_id);
        
        Ok(ResponseJson(ApiResponse::success_with_message(
            "Vault locked".to_string(),
            "Vault has been securely locked".to_string()
        )))
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

async fn vault_status(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<ResponseJson<ApiResponse<HashMap<String, bool>>>, StatusCode> {
    if let Some(claims) = auth::extract_claims(&headers, &state.jwt_manager) {
        let vaults = state.vaults.lock().unwrap();
        let is_unlocked = vaults.contains_key(&claims.user_id);
        
        let mut status = HashMap::new();
        status.insert("unlocked".to_string(), is_unlocked);
        
        Ok(ResponseJson(ApiResponse::success(status)))
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

async fn vault_stats(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<ResponseJson<ApiResponse<VaultStats>>, StatusCode> {
    if let Some(claims) = auth::extract_claims(&headers, &state.jwt_manager) {
        let vaults = state.vaults.lock().unwrap();
        if let Some(vault) = vaults.get(&claims.user_id) {
            if let Some(entries) = vault.get_entries() {
                let stats = calculate_vault_stats(entries);
                Ok(ResponseJson(ApiResponse::success(stats)))
            } else {
                Ok(ResponseJson(ApiResponse::error("Vault is locked".to_string())))
            }
        } else {
            Ok(ResponseJson(ApiResponse::error("Vault not found".to_string())))
        }
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

// Entry handlers
async fn get_entries(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<ResponseJson<ApiResponse<Vec<Entry>>>, StatusCode> {
    if let Some(claims) = auth::extract_claims(&headers, &state.jwt_manager) {
        let vaults = state.vaults.lock().unwrap();
        if let Some(vault) = vaults.get(&claims.user_id) {
            if let Some(entries) = vault.get_entries() {
                Ok(ResponseJson(ApiResponse::success(entries.clone())))
            } else {
                Ok(ResponseJson(ApiResponse::error("Vault is locked".to_string())))
            }
        } else {
            Ok(ResponseJson(ApiResponse::error("Vault not found".to_string())))
        }
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

async fn add_entry(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<AddEntryRequest>,
) -> Result<ResponseJson<ApiResponse<Entry>>, StatusCode> {
    if let Some(claims) = auth::extract_claims(&headers, &state.jwt_manager) {
        let mut vaults = state.vaults.lock().unwrap();
        if let Some(vault) = vaults.get_mut(&claims.user_id) {
            let entry = Entry::new(
                payload.service,
                payload.username,
                payload.password,
            );
            
            vault.add_entry_full(entry.clone());
            
            // Save to database
            let db = state.db.lock().unwrap();
            if let Err(e) = db.save_vault(&claims.user_id, vault) {
                log::error!("Failed to save vault: {}", e);
            }
            
            Ok(ResponseJson(ApiResponse::success_with_message(
                entry,
                "Entry added successfully".to_string()
            )))
        } else {
            Ok(ResponseJson(ApiResponse::error("Vault not found".to_string())))
        }
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

async fn update_entry(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(entry_id): Path<String>,
    Json(payload): Json<UpdateEntryRequest>,
) -> Result<ResponseJson<ApiResponse<Entry>>, StatusCode> {
    if let Some(claims) = auth::extract_claims(&headers, &state.jwt_manager) {
        let mut vaults = state.vaults.lock().unwrap();
        if let Some(vault) = vaults.get_mut(&claims.user_id) {
            if let Some(updated_entry) = vault.update_entry(&entry_id, payload) {
                // Save to database
                let db = state.db.lock().unwrap();
                if let Err(e) = db.save_vault(&claims.user_id, vault) {
                    log::error!("Failed to save vault: {}", e);
                }
                
                Ok(ResponseJson(ApiResponse::success_with_message(
                    updated_entry,
                    "Entry updated successfully".to_string()
                )))
            } else {
                Ok(ResponseJson(ApiResponse::error("Entry not found".to_string())))
            }
        } else {
            Ok(ResponseJson(ApiResponse::error("Vault not found".to_string())))
        }
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

async fn delete_entry(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(entry_id): Path<String>,
) -> Result<ResponseJson<ApiResponse<String>>, StatusCode> {
    if let Some(claims) = auth::extract_claims(&headers, &state.jwt_manager) {
        let mut vaults = state.vaults.lock().unwrap();
        if let Some(vault) = vaults.get_mut(&claims.user_id) {
            if vault.delete_entry(&entry_id) {
                // Save to database
                let db = state.db.lock().unwrap();
                if let Err(e) = db.save_vault(&claims.user_id, vault) {
                    log::error!("Failed to save vault: {}", e);
                }
                
                Ok(ResponseJson(ApiResponse::success_with_message(
                    "Entry deleted".to_string(),
                    "Entry deleted successfully".to_string()
                )))
            } else {
                Ok(ResponseJson(ApiResponse::error("Entry not found".to_string())))
            }
        } else {
            Ok(ResponseJson(ApiResponse::error("Vault not found".to_string())))
        }
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

async fn search_entries(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<SearchRequest>,
) -> Result<ResponseJson<ApiResponse<Vec<Entry>>>, StatusCode> {
    if let Some(claims) = auth::extract_claims(&headers, &state.jwt_manager) {
        let vaults = state.vaults.lock().unwrap();
        if let Some(vault) = vaults.get(&claims.user_id) {
            let results = vault.search_entries(&payload.query, payload.case_sensitive.unwrap_or(false));
            Ok(ResponseJson(ApiResponse::success(results)))
        } else {
            Ok(ResponseJson(ApiResponse::error("Vault not found".to_string())))
        }
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

// Utility handlers
async fn generate_password(
    Json(payload): Json<GeneratePasswordRequest>,
) -> Result<ResponseJson<ApiResponse<Vec<String>>>, StatusCode> {
    let passwords = (0..payload.count)
        .map(|_| crypto::generate_password(payload.length, payload.symbols))
        .collect();
    
    Ok(ResponseJson(ApiResponse::success(passwords)))
}

async fn check_password_strength(
    Json(password): Json<String>,
) -> Result<ResponseJson<ApiResponse<entry::PasswordStrengthInfo>>, StatusCode> {
    let strength = entry::analyze_password_strength(&password);
    Ok(ResponseJson(ApiResponse::success(strength)))
}

async fn get_audit_logs(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<ResponseJson<ApiResponse<Vec<security::AuditLogEntry>>>, StatusCode> {
    if let Some(claims) = auth::extract_claims(&headers, &state.jwt_manager) {
        let vaults = state.vaults.lock().unwrap();
        if let Some(vault) = vaults.get(&claims.user_id) {
            let logs = vault.get_audit_logs();
            Ok(ResponseJson(ApiResponse::success(logs)))
        } else {
            Ok(ResponseJson(ApiResponse::error("Vault not found".to_string())))
        }
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

async fn export_vault(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<ResponseJson<ApiResponse<String>>, StatusCode> {
    if let Some(claims) = auth::extract_claims(&headers, &state.jwt_manager) {
        let vaults = state.vaults.lock().unwrap();
        if let Some(vault) = vaults.get(&claims.user_id) {
            if let Some(entries) = vault.get_entries() {
                let export_data = serde_json::to_string_pretty(entries)
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                Ok(ResponseJson(ApiResponse::success(export_data)))
            } else {
                Ok(ResponseJson(ApiResponse::error("Vault is locked".to_string())))
            }
        } else {
            Ok(ResponseJson(ApiResponse::error("Vault not found".to_string())))
        }
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

async fn import_vault(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(import_data): Json<Vec<Entry>>,
) -> Result<ResponseJson<ApiResponse<String>>, StatusCode> {
    if let Some(claims) = auth::extract_claims(&headers, &state.jwt_manager) {
        let mut vaults = state.vaults.lock().unwrap();
        if let Some(vault) = vaults.get_mut(&claims.user_id) {
            for entry in import_data {
                vault.add_entry_full(entry);
            }
            
            // Save to database
            let db = state.db.lock().unwrap();
            if let Err(e) = db.save_vault(&claims.user_id, vault) {
                log::error!("Failed to save vault: {}", e);
            }
            
            Ok(ResponseJson(ApiResponse::success_with_message(
                "Import completed".to_string(),
                "Vault data imported successfully".to_string()
            )))
        } else {
            Ok(ResponseJson(ApiResponse::error("Vault not found".to_string())))
        }
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

async fn health_check() -> ResponseJson<ApiResponse<String>> {
    ResponseJson(ApiResponse::success("Server is healthy".to_string()))
}

// Helper functions
fn calculate_vault_stats(entries: &[Entry]) -> VaultStats {
    let mut categories_count = HashMap::new();
    let mut weak_passwords = 0;
    let mut duplicate_passwords = 0;
    let mut two_factor_enabled = 0;
    
    // Count categories and analyze passwords
    for entry in entries {
        let category = format!("{:?}", entry.category);
        *categories_count.entry(category).or_insert(0) += 1;
        
        if entry.password_strength.score < 3 {
            weak_passwords += 1;
        }
        
        if entry.two_factor_enabled.unwrap_or(false) {
            two_factor_enabled += 1;
        }
    }
    
    // Check for duplicate passwords
    let mut password_counts = HashMap::new();
    for entry in entries {
        *password_counts.entry(&entry.password).or_insert(0) += 1;
    }
    duplicate_passwords = password_counts.values().filter(|&&count| count > 1).count();
    
    let security_score = std::cmp::max(1, 10 - (weak_passwords + duplicate_passwords) * 10 / entries.len().max(1));
    
    VaultStats {
        total_entries: entries.len(),
        categories_count,
        weak_passwords,
        duplicate_passwords,
        old_passwords: 0, // Would need to implement age checking
        breached_passwords: 0, // Would need breach checking service
        two_factor_enabled,
        last_backup: chrono::Utc::now().to_rfc3339(),
        vault_size: format!("{}KB", (serde_json::to_string(entries).unwrap().len() / 1024).max(1)),
        security_score: security_score as u8,
    }
}
