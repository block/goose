use std::env;
use keyring::Entry;
use thiserror::Error;
#[cfg(test)]
use mockall::automock;
#[cfg(test)]
use mockall::predicate::*;

const KEYRING_SERVICE: &str = "goose";

#[derive(Error, Debug)]
pub enum KeyManagerError {
    #[error("Failed to access keyring: {0}")]
    KeyringAccess(String),

    #[error("Failed to save to keyring: {0}")]
    KeyringSave(String),

    #[error("Failed to access environment variable: {0}")]
    EnvVarAccess(String)
}

impl From<keyring::Error> for KeyManagerError {
    fn from(err: keyring::Error) -> Self {
        KeyManagerError::KeyringAccess(err.to_string())
    }
}

impl From<env::VarError> for KeyManagerError {
    fn from(err: env::VarError) -> Self {
        KeyManagerError::EnvVarAccess(err.to_string())
    }
}

// Define a trait for the keyring operations
#[cfg_attr(test, automock)]
pub trait Keyring: Send + Sync {
    fn get_password(&self) -> std::result::Result<String, KeyManagerError>;
    fn set_password(&self, password: &str) -> std::result::Result<(), KeyManagerError>;
}

#[cfg_attr(test, automock)]
pub trait Environment: Send + Sync {
    fn get_var(&self, key: &str) -> std::result::Result<String, env::VarError>;
    fn set_var(&self, key: &str, value: &str);
}

// Implement the trait for the actual environment
pub struct RealEnvironment;

impl Environment for RealEnvironment {
    fn get_var(&self, key: &str) -> std::result::Result<String, env::VarError> {
        env::var(key)
    }

    fn set_var(&self, key: &str, value: &str) {
        env::set_var(key, value)
    }
}

// Implement the trait for the actual keyring
impl Keyring for Entry {
    fn get_password(&self) -> std::result::Result<String, KeyManagerError> {
        self.get_password().map_err(KeyManagerError::from)
    }

    fn set_password(&self, password: &str) -> std::result::Result<(), KeyManagerError> {
        self.set_password(password).map_err(KeyManagerError::from)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum KeyRetrievalStrategy {
    /// Only look in environment variables
    EnvironmentOnly,
    /// Only look in system keyring
    KeyringOnly,
    /// Try keyring first, then environment variables (default behavior)
    Both,
}

impl Default for KeyRetrievalStrategy {
    fn default() -> Self {
        Self::Both
    }
}

pub fn get_api_key_default(
    api_key_name: &str,
    strategy: KeyRetrievalStrategy,
) -> Result<String, Box<dyn std::error::Error>> {
    let env = RealEnvironment;
    let kr = Entry::new(KEYRING_SERVICE, api_key_name)?;
    get_api_key(api_key_name, strategy, &kr, &env)
}

pub fn get_api_key(
    api_key_name: &str,
    strategy: KeyRetrievalStrategy,
    keyring: &impl Keyring,
    env: &impl Environment,
) -> Result<String, Box<dyn std::error::Error>> {
    match strategy {
        KeyRetrievalStrategy::EnvironmentOnly => {
            env.get_var(api_key_name)
                .map_err(|e| Box::new(KeyManagerError::from(e)) as Box<dyn std::error::Error>)
        }
        KeyRetrievalStrategy::KeyringOnly => {
            keyring.get_password()
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
        }
        KeyRetrievalStrategy::Both => {
            match keyring.get_password() {
                Ok(key) => Ok(key),
                Err(e) => {
                    println!("Note: Could not retrieve key from keyring: {}", e);
                    env.get_var(api_key_name).map_err(|_| {
                        Box::new(KeyManagerError::EnvVarAccess(format!(
                            "Could not find {} key in keyring or environment variables",
                            api_key_name
                        ))) as Box<dyn std::error::Error>
                    })
                }
            }
        }
    }
}

pub fn save_to_keyring(
    key_name: &str,
    api_key: &str,
) -> std::result::Result<(), KeyManagerError> {
    let kr = Entry::new(KEYRING_SERVICE, key_name)?;
    kr.set_password(api_key)
        .map_err(|e| KeyManagerError::KeyringSave(format!("Failed to save key {}: {}", key_name, e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_KEY: &str = "TEST_KEY";

    #[test]
    fn test_get_api_key_environment_only() {
        let mut mock_env = MockEnvironment::new();
        let mut mock_keyring = MockKeyring::new();
        
        mock_env.expect_get_var()
            .with(eq(TEST_KEY))
            .times(1)
            .return_once(|_| Ok("env_value".to_string()));
        
        mock_keyring.expect_get_password()
            .times(0);

        let result = get_api_key(
            TEST_KEY,
            KeyRetrievalStrategy::EnvironmentOnly,
            &mock_keyring,
            &mock_env,
        );
        
        assert!(matches!(result.as_deref(), Ok("env_value")));
    }

    #[test]
    fn test_get_api_key_environment_only_missing() {
        let mut mock_env = MockEnvironment::new();
        let mut mock_keyring = MockKeyring::new();
        
        mock_env.expect_get_var()
            .with(eq(TEST_KEY))
            .times(1)
            .return_once(|_| Err(env::VarError::NotPresent));
        
        mock_keyring.expect_get_password()
            .times(0);

        let result = get_api_key(
            TEST_KEY,
            KeyRetrievalStrategy::EnvironmentOnly,
            &mock_keyring,
            &mock_env,
        );
        
        assert!(result.is_err());
    }

    #[test]
    fn test_get_api_key_keyring_only() {
        let mut mock_env = MockEnvironment::new();
        let mut mock_keyring = MockKeyring::new();
        
        mock_keyring.expect_get_password()
            .times(1)
            .return_once(|| Ok("keyring_value".to_string()));
            
        mock_env.expect_get_var()
            .times(0);

        let result = get_api_key(
            TEST_KEY,
            KeyRetrievalStrategy::KeyringOnly,
            &mock_keyring,
            &mock_env,
        );
        
        assert!(matches!(result.as_deref(), Ok("keyring_value")));
    }

    #[test]
    fn test_get_api_key_keyring_only_missing() {
        let mut mock_env = MockEnvironment::new();
        let mut mock_keyring = MockKeyring::new();
        
        mock_keyring.expect_get_password()
            .times(1)
            .return_once(|| Err(KeyManagerError::KeyringAccess("Not found".to_string())));
            
        mock_env.expect_get_var()
            .times(0);

        let result = get_api_key(
            TEST_KEY,
            KeyRetrievalStrategy::KeyringOnly,
            &mock_keyring,
            &mock_env,
        );
        
        assert!(result.is_err());
    }

    #[test]
    fn test_get_api_key_both_keyring_succeeds() {
        let mut mock_env = MockEnvironment::new();
        let mut mock_keyring = MockKeyring::new();
        
        mock_keyring.expect_get_password()
            .times(1)
            .return_once(|| Ok("keyring_value".to_string()));
            
        mock_env.expect_get_var()
            .times(0);

        let result = get_api_key(
            TEST_KEY,
            KeyRetrievalStrategy::Both,
            &mock_keyring,
            &mock_env,
        );
        
        assert!(matches!(result.as_deref(), Ok("keyring_value")));
    }

    #[test]
    fn test_get_api_key_both_keyring_fails_env_succeeds() {
        let mut mock_env = MockEnvironment::new();
        let mut mock_keyring = MockKeyring::new();
        
        mock_keyring.expect_get_password()
            .times(1)
            .return_once(|| Err(KeyManagerError::KeyringAccess("Failed".to_string())));
            
        mock_env.expect_get_var()
            .with(eq(TEST_KEY))
            .times(1)
            .return_once(|_| Ok("env_value".to_string()));

        let result = get_api_key(
            TEST_KEY,
            KeyRetrievalStrategy::Both,
            &mock_keyring,
            &mock_env,
        );
        
        assert!(matches!(result.as_deref(), Ok("env_value")));
    }

    #[test]
    fn test_get_api_key_both_all_fail() {
        let mut mock_env = MockEnvironment::new();
        let mut mock_keyring = MockKeyring::new();
        
        mock_keyring.expect_get_password()
            .times(1)
            .return_once(|| Err(KeyManagerError::KeyringAccess("Failed".to_string())));
            
        mock_env.expect_get_var()
            .with(eq(TEST_KEY))
            .times(1)
            .return_once(|_| Err(env::VarError::NotPresent));

        let result = get_api_key(
            TEST_KEY,
            KeyRetrievalStrategy::Both,
            &mock_keyring,
            &mock_env,
        );
        
        assert!(result.is_err());
    }

    #[test]
    fn test_save_to_keyring() {
        let mut mock_keyring = MockKeyring::new();
        mock_keyring.expect_set_password()
            .with(eq("test_value"))
            .times(1)
            .return_once(|_| Ok(()));

        let result = save_to_keyring(TEST_KEY, "test_value");
        assert!(result.is_ok());
    }

    #[test]
    fn test_save_to_keyring_fails() {
        let mut mock_keyring = MockKeyring::new();
        mock_keyring.expect_set_password()
            .with(eq("test_value"))
            .times(1)
            .return_once(|_| Err(KeyManagerError::KeyringSave("Failed to save".to_string())));

        let result = save_to_keyring(TEST_KEY, "test_value");
        assert!(matches!(result.unwrap_err(), KeyManagerError::KeyringSave(_)));
    }
}
