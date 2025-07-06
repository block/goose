use super::{KeyringBackend, KeyringError};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Clone, Default)]
pub struct MockKeyringBackend {
    storage: Arc<RwLock<HashMap<String, String>>>,
}

impl MockKeyringBackend {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&self) {
        self.storage
            .write()
            .expect("Mock keyring lock poisoned")
            .clear();
    }

    pub fn contains(&self, service: &str, username: &str) -> bool {
        let key = format!("{}:{}", service, username);
        self.storage
            .read()
            .expect("Mock keyring lock poisoned")
            .contains_key(&key)
    }

    fn make_key(service: &str, username: &str) -> String {
        format!("{}:{}", service, username)
    }
}

impl KeyringBackend for MockKeyringBackend {
    fn get_password(&self, service: &str, username: &str) -> Result<String> {
        let key = Self::make_key(service, username);
        let storage = self
            .storage
            .read()
            .map_err(|e| KeyringError::Backend(format!("Mock keyring lock poisoned: {}", e)))?;

        storage
            .get(&key)
            .cloned()
            .ok_or_else(|| KeyringError::NotFound {
                service: service.to_string(),
                username: username.to_string(),
            })
            .map_err(anyhow::Error::from)
    }

    fn set_password(&self, service: &str, username: &str, password: &str) -> Result<()> {
        let key = Self::make_key(service, username);
        self.storage
            .write()
            .map_err(|e| KeyringError::Backend(format!("Mock keyring lock poisoned: {}", e)))?
            .insert(key, password.to_string());
        Ok(())
    }

    fn delete_password(&self, service: &str, username: &str) -> Result<()> {
        let key = Self::make_key(service, username);
        self.storage
            .write()
            .map_err(|e| KeyringError::Backend(format!("Mock keyring lock poisoned: {}", e)))?
            .remove(&key);
        Ok(())
    }
}
