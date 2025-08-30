use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub user_id: String,
    pub username: String,
    pub exp: usize,
    pub iat: usize,
}

pub struct JwtManager {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl JwtManager {
    pub fn new() -> Self {
        // In production, use a secure secret from environment variables
        let secret = std::env::var("JWT_SECRET")
            .unwrap_or_else(|_| "your-super-secret-jwt-key-change-this-in-production".to_string());
        
        Self {
            encoding_key: EncodingKey::from_secret(secret.as_ref()),
            decoding_key: DecodingKey::from_secret(secret.as_ref()),
        }
    }

    pub fn generate_token(&self, user_id: &str, username: &str) -> Result<String, StatusCode> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;
        
        let claims = Claims {
            user_id: user_id.to_string(),
            username: username.to_string(),
            iat: now,
            exp: now + 24 * 60 * 60, // 24 hours
        };

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
    }

    pub fn verify_token(&self, token: &str) -> Result<Claims, StatusCode> {
        let validation = Validation::new(Algorithm::HS256);
        
        decode::<Claims>(token, &self.decoding_key, &validation)
            .map(|data| data.claims)
            .map_err(|_| StatusCode::UNAUTHORIZED)
    }
}

pub fn extract_claims(headers: &HeaderMap, jwt_manager: &JwtManager) -> Option<Claims> {
    headers
        .get("authorization")
        .and_then(|auth_header| auth_header.to_str().ok())
        .and_then(|auth_str| {
            if auth_str.starts_with("Bearer ") {
                Some(&auth_str[7..])
            } else {
                None
            }
        })
        .and_then(|token| jwt_manager.verify_token(token).ok())
}

pub async fn auth_middleware(
    State(state): State<crate::AppState>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let path = request.uri().path();
    
    // Skip authentication for public routes
    if path.starts_with("/api/auth/") || 
       path.starts_with("/api/health") || 
       path.starts_with("/api/generate-password") ||
       path.starts_with("/api/check-password-strength") ||
       path == "/" ||
       !path.starts_with("/api/") {
        return Ok(next.run(request).await);
    }

    // Verify JWT token for protected routes
    if extract_claims(&headers, &state.jwt_manager).is_some() {
        Ok(next.run(request).await)
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}
