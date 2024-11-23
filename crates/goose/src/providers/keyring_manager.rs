use std::env;
use std::io::{self, Write};
use keyring;

// Define a trait for the keyring operations
pub trait Keyring {
    fn get_password(&self) -> Result<String, io::Error>;
    fn set_password(&self, password: &str) -> Result<(), io::Error>;
}

// Implement the trait for the actual keyring
impl Keyring for keyring::Entry {
    fn get_password(&self) -> Result<String, io::Error> {
        self.get_password().map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
    }

    fn set_password(&self, password: &str) -> Result<(), io::Error> {
        self.set_password(password).map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
    }
}

pub struct KeyringManager {
    service_name: String,
    key_name: String,
}

impl KeyringManager {
    pub fn new(service: &str, key: &str) -> Self {
        Self {
            service_name: service.to_string(),
            key_name: key.to_string(),
        }
    }

    /// Retrieves the API key, checks the keyring and environment, and prompts
    /// the user if necessary, with an option to store the key in the keyring.
    pub fn retrieve_api_key<K: Keyring>(&self, kr: &K, provider_name: &str) -> Option<String> {
        if let Ok(api_key) = kr.get_password() {
            println!("{} API key found in keyring", provider_name);
            env::set_var(&self.key_name, &api_key);
            return Some(api_key);
        } else if let Ok(api_key) = env::var(&self.key_name) {
            println!("{} API key found in environment.", provider_name);
            self.prompt_to_save_key(kr, &api_key);
            return Some(api_key);
        } else {
            return self.prompt_for_key(kr, provider_name);
        }
        None
    }

    fn prompt_for_key<K: Keyring>(&self, kr: &K, provider_name: &str) -> Option<String> {
        print!("Please enter your {} API key: ", provider_name);
        io::stdout().flush().ok();
        let mut api_key = String::new();
        if io::stdin().read_line(&mut api_key).is_ok() {
            let api_key = api_key.trim().to_string();
            env::set_var(&self.key_name, &api_key);
            self.prompt_to_save_key(kr, &api_key);
            return Some(api_key);
        }
        None
    }

    fn prompt_to_save_key<K: Keyring>(&self, kr: &K, api_key: &str) {
        print!("Would you like to save this API key to the keyring for future sessions? (y/n): ");
        io::stdout().flush().ok();
        let mut save = String::new();
        if io::stdin().read_line(&mut save).is_ok() {
            if save.trim().to_lowercase() == "y" {
                if kr.set_password(api_key).is_ok() {
                    println!("API key saved to keyring successfully!");
                } else {
                    println!("Failed to save API key to keyring");
                }
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use mockall::{mock, predicate::*};
    use std::env;

    mock! {
        KeyringEntry {}

        impl Keyring for KeyringEntry {
            fn get_password(&self) -> std::io::Result<String>;
            fn set_password(&self, password: &str) -> std::io::Result<()>;
        }
    }

    #[tokio::test]
    async fn test_retrieve_api_key_keyring() {
        let mut mock_keyring_entry = MockKeyringEntry::new();

        // Mock the get_password method to return a specific string
        mock_keyring_entry
            .expect_get_password()
            .returning(|| Ok("mock_key_from_keyring".to_string()));

        let manager = KeyringManager::new("test_service", "test_key");

        let result = manager.retrieve_api_key(&mock_keyring_entry, "MockService");

        assert_eq!(result, Some("mock_key_from_keyring".to_string()));
    }

    #[tokio::test]
    async fn test_retrieve_api_key_environment_over_keyring() {
        let mut mock_keyring_entry = MockKeyringEntry::new();

        // Setup keyring mock to return a specific key
        mock_keyring_entry
            .expect_get_password()
            .returning(|| Ok("mock_key_from_keyring".to_string()));

        // Set environment key which should be prioritized
        env::set_var("test_key", "mock_key_from_env");

        let manager = KeyringManager::new("test_service", "test_key");
        let result = manager.retrieve_api_key(&mock_keyring_entry, "MockService");

        assert_eq!(result, Some("mock_key_from_env".to_string()));
        env::remove_var("test_key");
    }

    #[tokio::test]
    async fn test_failed_keyring_set_password() {
        let mut mock_keyring_entry = MockKeyringEntry::new();

        // Mock the failure of setting password
        mock_keyring_entry
            .expect_set_password()
            .returning(|_| Err(std::io::Error::new(std::io::ErrorKind::Other, "failed to set password")));

        // Simulate user adding password to environment
        env::set_var("test_key", "mock_key_entry_failure");

        let manager = KeyringManager::new("test_service", "test_key");

        let result = manager.retrieve_api_key(&mock_keyring_entry, "MockService");

        // Validate entry from environment
        assert_eq!(result, Some("mock_key_entry_failure".to_string()));
        env::remove_var("test_key");
    }

    #[tokio::test]
    async fn test_user_declines_to_save_key() {
        let fake_keyring = MockKeyringEntry::new();

        // Simulate a scenario where user opts not to save the key.
        env::set_var("test_key", "mock_key_declined_to_save");

        let manager = KeyringManager::new("test_service", "test_key");

        let result = manager.retrieve_api_key(&fake_keyring, "MockService");

        assert_eq!(result, Some("mock_key_declined_to_save".to_string()));
        env::remove_var("test_key");
    }
}
