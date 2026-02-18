use std::collections::HashMap;

pub const MAX_RETRIES: u32 = 3;
pub const DEFAULT_TIMEOUT: u64 = 30000;

pub static GLOBAL_CONFIG: &str = "default";

pub type UserId = String;
pub type TokenMap = HashMap<String, String>;

pub trait Validator {
    fn validate(&self) -> bool;
}

pub trait Serializable {
    fn serialize(&self) -> Vec<u8>;
}

pub use crate::config::AppConfig;
pub use crate::errors::{AuthError, ValidationError};

pub struct Claims {
    pub user_id: String,
}

pub fn process_item(item: &str) -> bool {
    !item.is_empty()
}
