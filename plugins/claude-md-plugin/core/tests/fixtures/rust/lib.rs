//! Token handling library for JWT operations.

use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// JWT claims structure.
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub user_id: String,
    pub role: Role,
    pub exp: u64,
    pub iat: u64,
}

/// User role enumeration.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum Role {
    Admin,
    User,
    Guest,
}

/// Token configuration.
pub struct TokenConfig {
    pub secret: String,
    pub expires_in_secs: u64,
}

/// Token validation errors.
#[derive(Debug, thiserror::Error)]
pub enum TokenError {
    #[error("Token has expired")]
    Expired,
    #[error("Invalid token: {0}")]
    Invalid(String),
}

/// Validates a JWT token and returns the claims.
///
/// # Arguments
/// * `token` - The JWT token string to validate
/// * `secret` - The secret key for verification
///
/// # Returns
/// * `Ok(Claims)` - The decoded claims on success
/// * `Err(TokenError)` - The error on failure
pub fn validate_token(token: &str, secret: &str) -> Result<Claims, TokenError> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| {
        if e.to_string().contains("expired") {
            TokenError::Expired
        } else {
            TokenError::Invalid(e.to_string())
        }
    })?;

    Ok(token_data.claims)
}

/// Generates a new JWT token.
pub fn generate_token(user_id: &str, role: Role, config: &TokenConfig) -> Result<String, TokenError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let claims = Claims {
        user_id: user_id.to_string(),
        role,
        exp: now + config.expires_in_secs,
        iat: now,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(config.secret.as_bytes()),
    )
    .map_err(|e| TokenError::Invalid(e.to_string()))
}

fn internal_helper(data: &str) -> bool {
    !data.is_empty()
}
