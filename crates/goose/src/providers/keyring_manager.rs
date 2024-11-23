use std::env;
use std::io::{self, Write};
use keyring;

#[cfg_attr(test, mockall::automock)]
pub trait StdinReader {
    fn read_line(&self) -> io::Result<String>;
}

pub struct RealStdinReader;

impl StdinReader for RealStdinReader {
    fn read_line(&self) -> io::Result<String> {
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        Ok(input)
    }
}

// Define a trait for the keyring operations
pub trait Keyring {
    fn get_password(&self) -> Result<String, io::Error>;
    fn set_password(&self, password: &str) -> Result<(), io::Error>;
}

#[cfg_attr(test, mockall::automock)]
pub trait Environment {
    fn get_var(&self, key: &str) -> Result<String, env::VarError>;
    fn set_var(&self, key: &str, value: &str);
}

// Implement the trait for the actual environment
pub struct RealEnvironment;

impl Environment for RealEnvironment {
    fn get_var(&self, key: &str) -> Result<String, env::VarError> {
        env::var(key)
    }

    fn set_var(&self, key: &str, value: &str) {
        env::set_var(key, value)
    }
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

    /// Retrieves the API key, checks the environment first, then keyring, and prompts
    /// the user if necessary, with an option to store the key in the keyring.
    pub fn retrieve_api_key<K: Keyring, E: Environment, S: StdinReader>(
        &self,
        kr: &K,
        env: &E,
        provider_name: &str,
        stdin: &S
    ) -> Option<String> {
        // First check keyring
        if let Ok(api_key) = kr.get_password() {
            println!("{} API key found in keyring", provider_name);
            env.set_var(&self.key_name, &api_key);
            return Some(api_key);
        }

        // Then check environment variable
        if let Ok(api_key) = env.get_var(&self.key_name) {
            println!("{} API key found in environment.", provider_name);
            self.prompt_to_save_key(kr, &api_key, stdin);
            return Some(api_key);
        }
        
        // Finally, prompt user for key
        self.prompt_for_key(kr, env, provider_name, stdin)
    }

    fn prompt_for_key<K: Keyring, E: Environment, S: StdinReader>(
        &self,
        kr: &K,
        env: &E,
        provider_name: &str,
        stdin: &S
    ) -> Option<String> {
        print!("Please enter your {} API key: ", provider_name);
        io::stdout().flush().ok();
        if let Ok(api_key) = stdin.read_line() {
            let api_key = api_key.trim().to_string();
            env.set_var(&self.key_name, &api_key);
            self.prompt_to_save_key(kr, &api_key, stdin);
            return Some(api_key);
        }
        None
    }

    fn prompt_to_save_key<K: Keyring, S: StdinReader>(&self, kr: &K, api_key: &str, stdin: &S) {
        print!("Would you like to save this API key to the keyring for future sessions? (y/n): ");
        io::stdout().flush().ok();
        if let Ok(save) = stdin.read_line() {
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
    use mockall::{mock, predicate};

    mock! {
        KeyringEntry {}
        impl Keyring for KeyringEntry {
            fn get_password(&self) -> std::io::Result<String>;
            fn set_password(&self, password: &str) -> std::io::Result<()>;
        }
    }

    const TEST_KEY_NAME: &str = "test_key";

    #[tokio::test]
    async fn test_retrieve_api_key_keyring() {
        let mut mock_keyring_entry = MockKeyringEntry::new();
        let mut mock_env = MockEnvironment::new();
        let mut mock_stdin = MockStdinReader::new();
        let expected_key = "mock_key_from_keyring";

        // Mock keyring to return the key
        mock_keyring_entry
            .expect_get_password()
            .returning(move || Ok(expected_key.to_string()));

        // Mock environment set for when key is found in keyring
        mock_env
            .expect_set_var()
            .with(predicate::eq(TEST_KEY_NAME), predicate::eq(expected_key))
            .returning(|_, _| ());

        // Environment should not be checked since keyring has the key
        mock_env
            .expect_get_var()
            .times(0)
            .returning(|_| Err(env::VarError::NotPresent));

        let manager = KeyringManager::new("test_service", TEST_KEY_NAME);
        let result = manager.retrieve_api_key(&mock_keyring_entry, &mock_env, "MockService", &mock_stdin);

        assert_eq!(result, Some(expected_key.to_string()));
    }

    #[tokio::test]
    async fn test_retrieve_api_key_environment_fallback() {
        let mut mock_keyring_entry = MockKeyringEntry::new();
        let mut mock_env = MockEnvironment::new();
        let mut mock_stdin = MockStdinReader::new();
        let env_key = "mock_key_from_env";

        // Mock keyring to fail first
        mock_keyring_entry
            .expect_get_password()
            .returning(|| Err(std::io::Error::new(std::io::ErrorKind::Other, "no password found")));

        // Then environment should be checked and return the key
        mock_env
            .expect_get_var()
            .with(predicate::eq(TEST_KEY_NAME))
            .returning(move |_| Ok(env_key.to_string()));

        // Mock user input for save prompt (answering "n")
        mock_stdin
            .expect_read_line()
            .returning(|| Ok("n\n".to_string()));

        // Mock keyring set password prompt
        mock_keyring_entry
            .expect_set_password()
            .times(0) // Won't be called since we answer "n"
            .returning(|_| Ok(()));

        let manager = KeyringManager::new("test_service", TEST_KEY_NAME);
        let result = manager.retrieve_api_key(&mock_keyring_entry, &mock_env, "MockService", &mock_stdin);

        assert_eq!(result, Some(env_key.to_string()));
    }

    #[tokio::test]
    async fn test_failed_keyring_and_environment() {
        let mut mock_keyring_entry = MockKeyringEntry::new();
        let mut mock_env = MockEnvironment::new();
        let mut mock_stdin = MockStdinReader::new();
        let test_input = "test_input_key";

        // Mock keyring to fail
        mock_keyring_entry
            .expect_get_password()
            .returning(|| Err(std::io::Error::new(std::io::ErrorKind::Other, "no password found")));

        // Mock environment to also fail
        mock_env
            .expect_get_var()
            .with(predicate::eq(TEST_KEY_NAME))
            .returning(|_| Err(env::VarError::NotPresent));

        // Mock user input for API key
        mock_stdin
            .expect_read_line()
            .returning(move || Ok(format!("{}\n", test_input)));

        // Mock user input for save prompt (answering "n")
        mock_stdin
            .expect_read_line()
            .returning(|| Ok("n\n".to_string()));

        // Mock environment set for when user enters a key
        mock_env
            .expect_set_var()
            .with(predicate::eq(TEST_KEY_NAME), predicate::eq(test_input))
            .returning(|_, _| ());

        let manager = KeyringManager::new("test_service", TEST_KEY_NAME);
        let result = manager.prompt_for_key(&mock_keyring_entry, &mock_env, "MockService", &mock_stdin);

        assert_eq!(result, Some(test_input.to_string()));
    }

    #[tokio::test]
    async fn test_keyring_success_skips_environment() {
        let mut mock_keyring_entry = MockKeyringEntry::new();
        let mut mock_env = MockEnvironment::new();
        let mut mock_stdin = MockStdinReader::new();
        let keyring_key = "mock_key_from_keyring";

        // Mock keyring to succeed
        mock_keyring_entry
            .expect_get_password()
            .returning(move || Ok(keyring_key.to_string()));

        // Mock environment set for when key is found in keyring
        mock_env
            .expect_set_var()
            .with(predicate::eq(TEST_KEY_NAME), predicate::eq(keyring_key))
            .returning(|_, _| ());

        // Environment get_var should never be called
        mock_env
            .expect_get_var()
            .times(0)
            .returning(|_| Ok("should_not_be_called".to_string()));

        let manager = KeyringManager::new("test_service", TEST_KEY_NAME);
        let result = manager.retrieve_api_key(&mock_keyring_entry, &mock_env, "MockService", &mock_stdin);

        assert_eq!(result, Some(keyring_key.to_string()));
    }
}