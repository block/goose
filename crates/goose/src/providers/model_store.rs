use once_cell::sync::Lazy;
use std::sync::Mutex;

/// A global store for the current model being used
pub static CURRENT_MODEL: Lazy<Mutex<Option<String>>> = Lazy::new(|| Mutex::new(None));

/// Set the current model in the global store
pub fn set_current_model(model: &str) {
    if let Ok(mut current_model) = CURRENT_MODEL.lock() {
        *current_model = Some(model.to_string());
    }
}

/// Get the current model from the global store
pub fn get_current_model() -> Option<String> {
    CURRENT_MODEL.lock().ok().and_then(|model| model.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_and_get_current_model() {
        // Set the model
        set_current_model("gpt-4o");

        // Get the model and verify
        let model = get_current_model();
        assert_eq!(model, Some("gpt-4o".to_string()));

        // Change the model
        set_current_model("claude-3.5-sonnet");

        // Get the updated model and verify
        let model = get_current_model();
        assert_eq!(model, Some("claude-3.5-sonnet".to_string()));
    }
}
