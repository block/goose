use std::env;
use std::io::{self, Write};
use keyring;
use thiserror::Error;


#[derive(Error, Debug)]
pub enum KeyringError {
    #[error("Failed to access keyring: {0}")]
    KeyringAccess(String),

    #[error("Environment variable error: {0}")]
    EnvVar(#[from] env::VarError),

    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("User input error: {0}")]
    UserInput(String),

    #[error("Failed to save to keyring: {0}")]
    KeyringSave(String),
}

impl From<keyring::Error> for KeyringError {
    fn from(err: keyring::Error) -> Self {
        KeyringError::KeyringAccess(err.to_string())
    }
}

#[cfg_attr(test, mockall::automock)]
pub trait StdinReader {
    fn read_line(&self) -> Result<String, KeyringError>;
}

pub struct RealStdinReader;

impl StdinReader for RealStdinReader {
    fn read_line(&self) -> Result<String, KeyringError> {
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        Ok(input)
    }
}

// Define a trait for the keyring operations
pub trait Keyring {
    fn get_password(&self) -> Result<String, KeyringError>;
    fn set_password(&self, password: &str) -> Result<(), KeyringError>;
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
    fn get_password(&self) -> Result<String, KeyringError> {
        self.get_password().map_err(KeyringError::from)
    }

    fn set_password(&self, password: &str) -> Result<(), KeyringError> {
        self.set_password(password).map_err(KeyringError::from)
    }
}

/// Retrieves the API key from various sources in order:
/// 1. Keyring
/// 2. Environment variable
/// 3. User prompt
///
/// If a key is found in the environment, the user will be prompted to save it to the keyring.
/// If no key is found, the user will be prompted to enter one and optionally save it.
pub fn retrieve_api_key<K: Keyring, E: Environment, S: StdinReader>(
    kr: &K,
    env: &E,
    _service_name: &str,
    key_name: &str,
    provider_name: &str,
    stdin: &S
) -> Option<String> {
    // First check keyring
    if let Ok(api_key) = kr.get_password() {
        println!("{} API key found in keyring", provider_name);
        env.set_var(key_name, &api_key);
        return Some(api_key);
    }

    // Then check environment variable
    if let Ok(api_key) = env.get_var(key_name) {
        println!("{} API key found in environment.", provider_name);
        prompt_to_save_key(kr, &api_key, stdin);
        return Some(api_key);
    }
    
    // Finally, prompt user for key
    prompt_for_key(kr, env, _service_name, key_name, provider_name, stdin)
}

fn prompt_for_key<K: Keyring, E: Environment, S: StdinReader>(
    kr: &K,
    env: &E,
    _service_name: &str,
    key_name: &str,
    provider_name: &str,
    stdin: &S
) -> Option<String> {
    print!("Please enter your {} API key: ", provider_name);
    io::stdout().flush().ok();
    if let Ok(api_key) = stdin.read_line() {
        let api_key = api_key.trim().to_string();
        env.set_var(key_name, &api_key);
        prompt_to_save_key(kr, &api_key, stdin);
        return Some(api_key);
    }
    None
}

fn prompt_to_save_key<K: Keyring, S: StdinReader>(kr: &K, api_key: &str, stdin: &S) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::{mock, predicate};

    mock! {
        KeyringEntry {}
        impl Keyring for KeyringEntry {
            fn get_password(&self) -> Result<String, KeyringError>;
            fn set_password(&self, password: &str) -> Result<(), KeyringError>;
        }
    }

    const TEST_SERVICE: &str = "test_service";
    const TEST_KEY_NAME: &str = "test_key";

    #[tokio::test]
    async fn test_retrieve_api_key_keyring() {
        let mut mock_keyring_entry = MockKeyringEntry::new();
        let mut mock_env = MockEnvironment::new();
        let mock_stdin = MockStdinReader::new();
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

        let result = retrieve_api_key(
            &mock_keyring_entry,
            &mock_env,
            TEST_SERVICE,
            TEST_KEY_NAME,
            "MockService",
            &mock_stdin
        );

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
            .returning(|| Err(KeyringError::KeyringAccess("no password found".to_string())));

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

        let result = retrieve_api_key(
            &mock_keyring_entry,
            &mock_env,
            TEST_SERVICE,
            TEST_KEY_NAME,
            "MockService",
            &mock_stdin
        );

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
            .returning(|| Err(KeyringError::KeyringAccess("no password found".to_string())));

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

        let result = prompt_for_key(
            &mock_keyring_entry,
            &mock_env,
            TEST_SERVICE,
            TEST_KEY_NAME,
            "MockService",
            &mock_stdin
        );

        assert_eq!(result, Some(test_input.to_string()));
    }

    #[tokio::test]
    async fn test_keyring_success_skips_environment() {
        let mut mock_keyring_entry = MockKeyringEntry::new();
        let mut mock_env = MockEnvironment::new();
        let mock_stdin = MockStdinReader::new();
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

        let result = retrieve_api_key(
            &mock_keyring_entry,
            &mock_env,
            TEST_SERVICE,
            TEST_KEY_NAME,
            "MockService",
            &mock_stdin
        );

        assert_eq!(result, Some(keyring_key.to_string()));
    }
}