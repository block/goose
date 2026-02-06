//! Nostr key management for model publishing

use anyhow::Result;
use nostr_sdk::prelude::*;
use std::path::PathBuf;

fn config_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
    Ok(home.join(".config").join("goose"))
}

/// Manages Nostr keys for publishing model availability
pub struct KeyManager {
    keys: Keys,
}

impl KeyManager {
    pub fn generate() -> Result<Self> {
        let keys = Keys::generate();
        Ok(Self { keys })
    }

    pub fn from_private_key(private_key: &str) -> Result<Self> {
        let secret_key = if private_key.starts_with("nsec") {
            SecretKey::from_bech32(private_key)?
        } else {
            SecretKey::from_hex(private_key)?
        };
        let keys = Keys::new(secret_key);
        Ok(Self { keys })
    }

    pub fn load_or_generate(path: &PathBuf) -> Result<Self> {
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            let private_key = content.trim();
            Self::from_private_key(private_key)
        } else {
            let manager = Self::generate()?;
            manager.save(path)?;
            Ok(manager)
        }
    }

    pub fn load_default_or_generate() -> Result<Self> {
        let key_path = config_dir()?.join("nostr-key.nsec");
        Self::load_or_generate(&key_path)
    }

    pub fn save(&self, path: &PathBuf) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let nsec = self.keys.secret_key().to_bech32()?;
        std::fs::write(path, &nsec)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(path)?.permissions();
            perms.set_mode(0o600);
            std::fs::set_permissions(path, perms)?;
        }

        Ok(())
    }

    pub fn npub(&self) -> String {
        self.keys.public_key().to_bech32().unwrap_or_default()
    }

    pub fn public_key_hex(&self) -> String {
        self.keys.public_key().to_hex()
    }

    pub fn nsec(&self) -> Result<String> {
        Ok(self.keys.secret_key().to_bech32()?)
    }

    pub fn keys(&self) -> &Keys {
        &self.keys
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_keys() {
        let manager = KeyManager::generate().unwrap();
        assert!(manager.npub().starts_with("npub"));
    }

    #[test]
    fn test_roundtrip_nsec() {
        let generated = KeyManager::generate().unwrap();
        let nsec = generated.nsec().unwrap();
        let loaded = KeyManager::from_private_key(&nsec).unwrap();
        assert_eq!(generated.npub(), loaded.npub());
    }
}
