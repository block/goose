//! Privacy-respecting client fingerprinting for telemetry.
//!
//! Generates a stable, hashed client ID plus OS/arch info for OTLP resource attributes.
//! The raw UUID is stored locally; only the SHA-256 hash is sent.

use opentelemetry::KeyValue;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::config::Config;

const CLIENT_ID_CONFIG_KEY: &str = "goose_client_id";

/// Get or create the client ID, returning its SHA-256 hash.
pub fn get_client_id_hash() -> String {
    let config = Config::global();

    let client_id: String = match config.get_param(CLIENT_ID_CONFIG_KEY) {
        Ok(id) => id,
        Err(_) => {
            let new_id = Uuid::new_v4().to_string();
            let _ = config.set_param(CLIENT_ID_CONFIG_KEY, &new_id);
            new_id
        }
    };

    let mut hasher = Sha256::new();
    hasher.update(client_id.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn get_os_type() -> &'static str {
    #[cfg(target_os = "macos")]
    return "macos";
    #[cfg(target_os = "linux")]
    return "linux";
    #[cfg(target_os = "windows")]
    return "windows";
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    return "unknown";
}

pub fn get_host_arch() -> &'static str {
    #[cfg(target_arch = "x86_64")]
    return "x86_64";
    #[cfg(target_arch = "aarch64")]
    return "aarch64";
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    return "unknown";
}

/// Create OTLP resource attributes for client fingerprinting.
pub fn create_fingerprint_attributes() -> Vec<KeyValue> {
    vec![
        KeyValue::new("client.id", get_client_id_hash()),
        KeyValue::new("os.type", get_os_type()),
        KeyValue::new("host.arch", get_host_arch()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_id_hash_is_stable() {
        let hash1 = get_client_id_hash();
        let hash2 = get_client_id_hash();
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64);
    }

    #[test]
    fn test_os_and_arch() {
        assert!(["macos", "linux", "windows", "unknown"].contains(&get_os_type()));
        assert!(["x86_64", "aarch64", "unknown"].contains(&get_host_arch()));
    }

    #[test]
    fn test_fingerprint_attributes() {
        let attrs = create_fingerprint_attributes();
        assert_eq!(attrs.len(), 3);
    }
}
