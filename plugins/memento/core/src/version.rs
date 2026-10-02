//! Runtime handshake, independent of the plugin's release version.
use serde_json::{Value, json};

pub fn information() -> Value {
    let store_format = crate::store::CURRENT_FORMAT;
    json!({
        "protocol_version": 1,
        "package_version": env!("CARGO_PKG_VERSION"),
        "build_identity": env!("MEMENTO_BUILD_ID"),
        "platform": {"os": std::env::consts::OS, "arch": std::env::consts::ARCH},
        "store_format": {
            "current": store_format,
            "read": {"min": 0, "max": store_format},
            "write": {"min": store_format, "max": store_format},
            "legacy": [0]
        },
        "capabilities": ["checkpoint", "store-status", "semantic-config-check", "migrate", "literal-search", "local-semantic"]
    })
}
