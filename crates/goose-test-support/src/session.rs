use std::sync::{Arc, Mutex};

pub const TEST_SESSION_ID: &str = "test-session-id";
pub const TEST_MODEL: &str = "gpt-5-nano";

const NOT_YET_SET: &str = "session-id-not-yet-set";
pub(crate) const SESSION_ID_HEADER: &str = "agent-session-id";

#[derive(Clone)]
pub struct ExpectedSessionId {
    value: Arc<Mutex<String>>,
    errors: Arc<Mutex<Vec<String>>>,
}

impl Default for ExpectedSessionId {
    fn default() -> Self {
        Self {
            value: Arc::new(Mutex::new(NOT_YET_SET.to_string())),
            errors: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl ExpectedSessionId {
    pub fn set(&self, id: impl Into<String>) {
        *self.value.lock().unwrap() = id.into();
    }

    pub fn validate(&self, actual: Option<&str>) -> Result<(), String> {
        let expected = self.value.lock().unwrap();
        let err = match actual {
            Some(act) if act == *expected => None,
            _ => Some(format!(
                "{} mismatch: expected '{}', got {:?}",
                SESSION_ID_HEADER, expected, actual
            )),
        };
        match err {
            Some(e) => {
                self.errors.lock().unwrap().push(e.clone());
                Err(e)
            }
            None => Ok(()),
        }
    }

    pub fn assert_matches(&self, actual: &str) {
        let result = self.validate(Some(actual));
        assert!(result.is_ok(), "{}", result.unwrap_err());
        let errors = self.errors.lock().unwrap();
        assert!(
            errors.is_empty(),
            "Session ID validation errors: {:?}",
            *errors
        );
    }
}
