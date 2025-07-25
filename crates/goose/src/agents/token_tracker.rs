use std::sync::Arc;
use tokio::sync::RwLock;

use crate::providers::base::{ProviderUsage, Usage};

#[derive(Debug, Clone)]
pub struct TokenTracker {
    pub current_usage: Usage,
    pub context_limit: Option<usize>,
    pub warning_80_shown: bool,
    pub warning_90_shown: bool,
}

impl Default for TokenTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenTracker {
    pub fn new() -> Self {
        Self {
            current_usage: Usage::default(),
            context_limit: None,
            warning_80_shown: false,
            warning_90_shown: false,
        }
    }

    pub fn update_usage(&mut self, usage: &ProviderUsage) {
        self.current_usage += usage.usage;
    }

    pub fn set_context_limit(&mut self, limit: usize) {
        self.context_limit = Some(limit);
    }

    pub fn usage_percentage(&self) -> Option<f64> {
        match (self.current_usage.total_tokens, self.context_limit) {
            (Some(used), Some(limit)) => Some((used as f64 / limit as f64) * 100.0),
            _ => None,
        }
    }

    pub fn check_warning(&mut self) -> Option<String> {
        let percentage = self.usage_percentage()?;

        if percentage >= 90.0 && !self.warning_90_shown {
            self.warning_90_shown = true;
            let (used, limit) = self.token_counts()?;
            Some(format!(
                "WARNING: Approaching context limit! Used {} of {} tokens ({}%)",
                used, limit, percentage as i32
            ))
        } else if percentage >= 80.0 && !self.warning_80_shown {
            self.warning_80_shown = true;
            let (used, limit) = self.token_counts()?;
            Some(format!(
                "Context usage at {}% ({} of {} tokens)",
                percentage as i32, used, limit
            ))
        } else {
            None
        }
    }

    pub fn token_counts(&self) -> Option<(i32, usize)> {
        match (self.current_usage.total_tokens, self.context_limit) {
            (Some(used), Some(limit)) => Some((used, limit)),
            _ => None,
        }
    }

    pub fn status(&self) -> String {
        match self.token_counts() {
            Some((used, limit)) => {
                let percentage = self.usage_percentage().unwrap_or(0.0);
                format!("Token usage: {} / {} ({}%)", used, limit, percentage as i32)
            }
            None => match self.current_usage.total_tokens {
                Some(used) => format!("Token usage: {} (no limit available)", used),
                None => "Token usage: unavailable".to_string(),
            },
        }
    }

    pub fn reset(&mut self) {
        self.current_usage = Usage::default();
        self.warning_80_shown = false;
        self.warning_90_shown = false;
    }
}

pub type SharedTokenTracker = Arc<RwLock<TokenTracker>>;

pub fn create_shared_tracker() -> SharedTokenTracker {
    Arc::new(RwLock::new(TokenTracker::new()))
}
