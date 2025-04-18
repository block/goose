use std::time::{Duration, Instant};

const DISABLE_SUGGESTION_COOLDOWN: Duration = Duration::from_secs(3600);
const MAX_DISABLE_SUGGESTIONS: u32 = 3;

/// Manages the suggestion state and logic for disabling extensions
pub struct ExtensionManagerSuggestions {
    last_disable_suggestion: Option<Instant>,
    disable_suggestion_cooldown: Duration,
    disable_suggestion_count: u32,
    max_disable_suggestions: u32,
}

impl ExtensionManagerSuggestions {
    pub fn new() -> Self {
        Self {
            last_disable_suggestion: None,
            disable_suggestion_cooldown: DISABLE_SUGGESTION_COOLDOWN,
            disable_suggestion_count: 0,
            max_disable_suggestions: MAX_DISABLE_SUGGESTIONS,
        }
    }

    /// Check if we should show a suggestion based on count and cooldown
    pub fn should_show_disable_suggestion(&self) -> bool {
        // Check max suggestions limit
        if self.disable_suggestion_count >= self.max_disable_suggestions {
            return false;
        }

        // Check cooldown
        match self.last_disable_suggestion {
            None => true,
            Some(last_time) => last_time.elapsed() >= self.disable_suggestion_cooldown,
        }
    }

    /// Record that a suggestion was shown
    pub fn record_disable_suggestion(&mut self) {
        self.disable_suggestion_count += 1;
        self.last_disable_suggestion = Some(Instant::now());
    }

    /// Set a custom cooldown duration
    pub fn set_disable_suggestion_cooldown(&mut self, duration: Duration) {
        self.disable_suggestion_cooldown = duration;
    }

    /// Set a custom max suggestions limit
    pub fn set_max_disable_suggestions(&mut self, max: u32) {
        self.max_disable_suggestions = max;
    }
}

impl Default for ExtensionManagerSuggestions {
    fn default() -> Self {
        Self::new()
    }
}
