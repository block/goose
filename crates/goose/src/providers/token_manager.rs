use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};

#[derive(Clone, Debug)]
pub struct AccessToken {
    pub token: String,
    pub expires_at: SystemTime,
}

impl AccessToken {
    pub fn new(token: String, expires_in_seconds: u64) -> Self {
        Self {
            token,
            expires_at: SystemTime::now() + Duration::from_secs(expires_in_seconds),
        }
    }

    pub fn is_expired(&self) -> bool {
        SystemTime::now() >= self.expires_at
    }
}

#[derive(Debug)]
pub struct TokenManager {
    token: Arc<RwLock<Option<AccessToken>>>,
}

impl TokenManager {
    pub fn new() -> Self {
        Self {
            token: Arc::new(RwLock::new(None)),
        }
    }

    pub fn set_token(&self, token: String, expires_in_seconds: u64) {
        let access_token = AccessToken::new(token, expires_in_seconds);
        let mut current_token = self.token.write().unwrap();
        *current_token = Some(access_token);
    }

    pub fn is_expired(&self) -> bool {
        let token = self.token.read().unwrap();
        match &*token {
            None => true, // No token means expired
            Some(access_token) => access_token.is_expired(),
        }
    }

    pub fn get_token(&self) -> Option<String> {
        let token = self.token.read().unwrap();
        match &*token {
            None => None,
            Some(access_token) => {
                if access_token.is_expired() {
                    None
                } else {
                    Some(access_token.token.clone())
                }
            }
        }
    }
}

impl Clone for TokenManager {
    fn clone(&self) -> Self {
        Self {
            token: Arc::clone(&self.token),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_access_token_new() {
        let token = AccessToken::new("test_token".to_string(), 3600);
        assert_eq!(token.token, "test_token");
        assert!(!token.is_expired());
    }

    #[test]
    fn test_access_token_not_expired() {
        let token = AccessToken::new("test_token".to_string(), 3600);
        assert!(!token.is_expired());
    }

    #[test]
    fn test_access_token_expired() {
        let token = AccessToken::new("test_token".to_string(), 0);
        thread::sleep(Duration::from_millis(10));
        assert!(token.is_expired());
    }

    #[test]
    fn test_access_token_expires_at_boundary() {
        let token = AccessToken::new("test_token".to_string(), 0);
        assert!(token.is_expired());
    }

    #[test]
    fn test_token_manager_new() {
        let manager = TokenManager::new();
        assert!(manager.is_expired());
        assert_eq!(manager.get_token(), None);
    }

    #[test]
    fn test_token_manager_set_token() {
        let manager = TokenManager::new();
        manager.set_token("my_token".to_string(), 3600);
        assert!(!manager.is_expired());
        assert_eq!(manager.get_token(), Some("my_token".to_string()));
    }

    #[test]
    fn test_token_manager_is_expired_no_token() {
        let manager = TokenManager::new();
        assert!(manager.is_expired());
    }

    #[test]
    fn test_token_manager_is_expired_with_valid_token() {
        let manager = TokenManager::new();
        manager.set_token("valid_token".to_string(), 3600);
        assert!(!manager.is_expired());
    }

    #[test]
    fn test_token_manager_is_expired_with_expired_token() {
        let manager = TokenManager::new();
        manager.set_token("expired_token".to_string(), 0);
        thread::sleep(Duration::from_millis(10));
        assert!(manager.is_expired());
    }

    #[test]
    fn test_token_manager_get_token_none() {
        let manager = TokenManager::new();
        assert_eq!(manager.get_token(), None);
    }

    #[test]
    fn test_token_manager_get_token_valid() {
        let manager = TokenManager::new();
        manager.set_token("valid_token".to_string(), 3600);
        assert_eq!(manager.get_token(), Some("valid_token".to_string()));
    }

    #[test]
    fn test_token_manager_get_token_expired_returns_none() {
        let manager = TokenManager::new();
        manager.set_token("expired_token".to_string(), 0);
        thread::sleep(Duration::from_millis(10));
        assert_eq!(manager.get_token(), None);
    }

    #[test]
    fn test_token_manager_replace_token() {
        let manager = TokenManager::new();
        manager.set_token("token1".to_string(), 3600);
        assert_eq!(manager.get_token(), Some("token1".to_string()));
        
        manager.set_token("token2".to_string(), 3600);
        assert_eq!(manager.get_token(), Some("token2".to_string()));
    }

    #[test]
    fn test_token_manager_clone_shares_state() {
        let manager1 = TokenManager::new();
        manager1.set_token("shared_token".to_string(), 3600);
        
        let manager2 = manager1.clone();
        assert_eq!(manager2.get_token(), Some("shared_token".to_string()));
        
        manager2.set_token("new_token".to_string(), 3600);
        assert_eq!(manager1.get_token(), Some("new_token".to_string()));
    }

    #[test]
    fn test_token_manager_thread_safety() {
        let manager = TokenManager::new();
        manager.set_token("initial_token".to_string(), 3600);
        
        let manager_clone = manager.clone();
        let handle = thread::spawn(move || {
            manager_clone.set_token("thread_token".to_string(), 3600);
        });
        
        handle.join().unwrap();
        assert_eq!(manager.get_token(), Some("thread_token".to_string()));
    }

    #[test]
    fn test_access_token_clone() {
        let token1 = AccessToken::new("test_token".to_string(), 3600);
        let token2 = token1.clone();
        assert_eq!(token1.token, token2.token);
        assert_eq!(token1.expires_at, token2.expires_at);
    }

    #[test]
    fn test_token_manager_long_token_string() {
        let manager = TokenManager::new();
        let long_token = "a".repeat(10000);
        manager.set_token(long_token.clone(), 3600);
        assert_eq!(manager.get_token(), Some(long_token));
    }

    #[test]
    fn test_token_manager_empty_token_string() {
        let manager = TokenManager::new();
        manager.set_token("".to_string(), 3600);
        assert_eq!(manager.get_token(), Some("".to_string()));
        assert!(!manager.is_expired());
    }

    #[test]
    fn test_token_manager_zero_expiry() {
        let manager = TokenManager::new();
        manager.set_token("zero_expiry".to_string(), 0);
        assert!(manager.is_expired());
    }
}
