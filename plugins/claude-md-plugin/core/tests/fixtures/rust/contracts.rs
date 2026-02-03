//! Contract examples for Rust code analyzer.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Error types for token operations.
#[derive(Debug, Error)]
pub enum TokenError {
    #[error("invalid token")]
    Invalid,
    #[error("token expired")]
    Expired,
}

/// Claims extracted from a JWT token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub user_id: String,
    pub role: String,
}

/// Validates a JWT token.
///
/// # Arguments
/// * `token` - Must be non-empty
///
/// # Returns
/// Ok(Claims) with valid userId
///
/// # Errors
/// - `TokenError::Invalid` if malformed
/// - `TokenError::Expired` if expired
pub fn validate_token(token: &str) -> Result<Claims, TokenError> {
    if token.is_empty() {
        return Err(TokenError::Invalid);
    }

    // ... validation logic
    Ok(Claims {
        user_id: "user123".to_string(),
        role: "admin".to_string(),
    })
}

/// Order for processing.
pub struct Order {
    pub id: String,
    pub items: Vec<String>,
}

/// Receipt from order processing.
pub struct Receipt {
    pub order_id: String,
    pub total: u32,
}

/// Processes an order.
///
/// # Arguments
/// * `order` - Order with id and items
///
/// # Returns
/// Receipt with orderId and total
///
/// # Errors
/// Returns error if validation fails
pub fn process_order(order: &Order) -> Result<Receipt, String> {
    if order.id.is_empty() {
        return Err("order ID required".to_string());
    }
    if order.items.is_empty() {
        return Err("items required".to_string());
    }

    Ok(Receipt {
        order_id: order.id.clone(),
        total: 100,
    })
}

/// Internal helper function.
fn internal_helper(data: &str) -> String {
    data.trim().to_string()
}
