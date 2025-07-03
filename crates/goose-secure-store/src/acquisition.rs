use crate::{KeyringSecureStore, Result, SecretError, SecureStore};
use console::Term;
use std::io::Write;

/// Handles the acquisition of secrets using various methods
pub struct SecretAcquisition {
    store: Box<dyn SecureStore>,
}

impl SecretAcquisition {
    /// Create a new SecretAcquisition instance with the default keyring store
    pub fn new() -> Self {
        Self {
            store: Box::new(KeyringSecureStore::new()),
        }
    }

    /// Create a new SecretAcquisition instance with a custom store
    pub fn with_store(store: Box<dyn SecureStore>) -> Self {
        Self { store }
    }

    /// Acquire a secret using the prompt method
    ///
    /// # Arguments
    /// * `server_name` - The name of the MCP server
    /// * `secret_name` - The name of the secret
    /// * `description` - Description of what the secret is used for
    /// * `prompt_message` - Optional custom prompt message
    pub fn acquire_prompt_secret(
        &self,
        server_name: &str,
        secret_name: &str,
        description: &str,
        prompt_message: Option<&str>,
    ) -> Result<String> {
        let service_name = KeyringSecureStore::create_service_name(server_name, Some(secret_name));

        // Check if secret already exists
        if self.store.has_secret(&service_name, secret_name) {
            return self.store.get_secret(&service_name, secret_name);
        }

        // Prompt user for consent before storing
        if !self.prompt_for_consent(server_name, secret_name, description)? {
            return Err(SecretError::UserCancelled);
        }

        // Get the secret from user input
        let secret = self.prompt_for_secret(secret_name, description, prompt_message)?;

        // Store the secret
        self.store.set_secret(&service_name, secret_name, &secret)?;

        Ok(secret)
    }

    /// Get an existing secret from the store
    ///
    /// # Arguments
    /// * `server_name` - The name of the MCP server
    /// * `secret_name` - The name of the secret
    pub fn get_secret(&self, server_name: &str, secret_name: &str) -> Result<String> {
        let service_name = KeyringSecureStore::create_service_name(server_name, Some(secret_name));
        self.store.get_secret(&service_name, secret_name)
    }

    /// Check if a secret exists
    ///
    /// # Arguments
    /// * `server_name` - The name of the MCP server
    /// * `secret_name` - The name of the secret
    pub fn has_secret(&self, server_name: &str, secret_name: &str) -> bool {
        let service_name = KeyringSecureStore::create_service_name(server_name, Some(secret_name));
        self.store.has_secret(&service_name, secret_name)
    }

    /// Delete a secret from the store
    ///
    /// # Arguments
    /// * `server_name` - The name of the MCP server
    /// * `secret_name` - The name of the secret
    pub fn delete_secret(&self, server_name: &str, secret_name: &str) -> Result<()> {
        let service_name = KeyringSecureStore::create_service_name(server_name, Some(secret_name));
        self.store.delete_secret(&service_name, secret_name)
    }

    /// Prompt user for consent to store a secret
    fn prompt_for_consent(
        &self,
        server_name: &str,
        secret_name: &str,
        description: &str,
    ) -> Result<bool> {
        let term = Term::stdout();

        // Check if we're in a TTY environment
        if !term.is_term() {
            // In non-TTY environments, we can't prompt, so we assume consent
            // This allows for automated/scripted usage
            return Ok(true);
        }

        println!("\n🔐 Secret Storage Consent");
        println!("─────────────────────────");
        println!("Server: {}", server_name);
        println!("Secret: {} ({})", secret_name, description);
        println!("\nGoose would like to securely store this secret in your system's keychain.");
        println!("This will allow automatic retrieval for future MCP server connections.");

        loop {
            print!("\nDo you consent to storing this secret? [y/N]: ");
            std::io::stdout()
                .flush()
                .map_err(|e| SecretError::Other(format!("IO error: {}", e)))?;

            let input = term
                .read_line()
                .map_err(|e| SecretError::Other(format!("Failed to read input: {}", e)))?;
            let input = input.trim().to_lowercase();

            match input.as_str() {
                "y" | "yes" => return Ok(true),
                "n" | "no" | "" => return Ok(false),
                _ => println!("Please enter 'y' for yes or 'n' for no."),
            }
        }
    }

    /// Prompt user for secret input
    fn prompt_for_secret(
        &self,
        secret_name: &str,
        description: &str,
        prompt_message: Option<&str>,
    ) -> Result<String> {
        let term = Term::stdout();

        // Check if we're in a TTY environment
        if !term.is_term() {
            return Err(SecretError::Other(
                "Cannot prompt for secret in non-TTY environment. Please set the environment variable directly.".to_string()
            ));
        }

        println!("\n🔑 Secret Input Required");
        println!("─────────────────────────");

        let default_message = format!("Please enter your {} ({})", secret_name, description);
        let message = prompt_message.unwrap_or(&default_message);
        println!("{}", message);

        loop {
            print!("\n{}: ", secret_name);
            std::io::stdout()
                .flush()
                .map_err(|e| SecretError::Other(format!("IO error: {}", e)))?;

            let secret = term
                .read_secure_line()
                .map_err(|e| SecretError::Other(format!("Failed to read secret: {}", e)))?;

            if secret.trim().is_empty() {
                println!("Secret cannot be empty. Please try again.");
                continue;
            }

            return Ok(secret);
        }
    }
}

impl Default for SecretAcquisition {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    // Simple mock store for acquisition tests
    struct TestMockStore {
        storage: Mutex<HashMap<String, String>>,
    }

    impl TestMockStore {
        fn new() -> Self {
            Self {
                storage: Mutex::new(HashMap::new()),
            }
        }

        fn make_key(service: &str, username: &str) -> String {
            format!("{}:{}", service, username)
        }
    }

    impl SecureStore for TestMockStore {
        fn set_secret(&self, service: &str, username: &str, secret: &str) -> Result<()> {
            let key = Self::make_key(service, username);
            let mut storage = self.storage.lock().unwrap();
            storage.insert(key, secret.to_string());
            Ok(())
        }

        fn get_secret(&self, service: &str, username: &str) -> Result<String> {
            let key = Self::make_key(service, username);
            let storage = self.storage.lock().unwrap();
            storage
                .get(&key)
                .cloned()
                .ok_or_else(|| SecretError::NotFound(format!("Secret not found: {}", key)))
        }

        fn delete_secret(&self, service: &str, username: &str) -> Result<()> {
            let key = Self::make_key(service, username);
            let mut storage = self.storage.lock().unwrap();
            storage
                .remove(&key)
                .ok_or_else(|| SecretError::NotFound(format!("Secret not found: {}", key)))?;
            Ok(())
        }

        fn has_secret(&self, service: &str, username: &str) -> bool {
            let key = Self::make_key(service, username);
            let storage = self.storage.lock().unwrap();
            storage.contains_key(&key)
        }
    }

    #[test]
    fn test_get_existing_secret() {
        let mock_store = TestMockStore::new();
        let service_name = KeyringSecureStore::create_service_name("test_server", Some("api_key"));
        mock_store
            .set_secret(&service_name, "api_key", "test_secret")
            .unwrap();

        let acquisition = SecretAcquisition::with_store(Box::new(mock_store));
        let result = acquisition.get_secret("test_server", "api_key").unwrap();

        assert_eq!(result, "test_secret");
    }

    #[test]
    fn test_has_secret() {
        let mock_store = TestMockStore::new();
        let service_name = KeyringSecureStore::create_service_name("test_server", Some("api_key"));
        mock_store
            .set_secret(&service_name, "api_key", "test_secret")
            .unwrap();

        let acquisition = SecretAcquisition::with_store(Box::new(mock_store));

        assert!(acquisition.has_secret("test_server", "api_key"));
        assert!(!acquisition.has_secret("test_server", "other_key"));
    }

    #[test]
    fn test_delete_secret() {
        let mock_store = TestMockStore::new();
        let service_name = KeyringSecureStore::create_service_name("test_server", Some("api_key"));
        mock_store
            .set_secret(&service_name, "api_key", "test_secret")
            .unwrap();

        let acquisition = SecretAcquisition::with_store(Box::new(mock_store));

        assert!(acquisition.has_secret("test_server", "api_key"));
        acquisition.delete_secret("test_server", "api_key").unwrap();
        assert!(!acquisition.has_secret("test_server", "api_key"));
    }
}
