use super::{KeyringBackend, KeyringError};
use anyhow::Result;
use async_trait::async_trait;
use keyring::Entry;

pub struct SystemKeyringBackend;

#[async_trait]
impl KeyringBackend for SystemKeyringBackend {
    async fn get_password(&self, service: &str, username: &str) -> Result<String> {
        let entry =
            Entry::new(service, username).map_err(|e| KeyringError::Backend(e.to_string()))?;

        entry
            .get_password()
            .map_err(|e| match e {
                keyring::Error::NoEntry => KeyringError::NotFound {
                    service: service.to_string(),
                    username: username.to_string(),
                },
                _ => KeyringError::Backend(e.to_string()),
            })
            .map_err(anyhow::Error::from)
    }

    async fn set_password(&self, service: &str, username: &str, password: &str) -> Result<()> {
        let entry =
            Entry::new(service, username).map_err(|e| KeyringError::Backend(e.to_string()))?;

        entry
            .set_password(password)
            .map_err(|e| KeyringError::Backend(e.to_string()))
            .map_err(anyhow::Error::from)
    }

    async fn delete_password(&self, service: &str, username: &str) -> Result<()> {
        let entry =
            Entry::new(service, username).map_err(|e| KeyringError::Backend(e.to_string()))?;

        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()), // Already deleted is fine
            Err(e) => Err(anyhow::Error::from(KeyringError::Backend(e.to_string()))),
        }
    }
}
