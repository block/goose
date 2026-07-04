use crate::config::Config;

/// Runtime mode for the Context Manager, controlled by `GOSLING_CONTEXT_MANAGER`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ContextManagerMode {
    /// Current Gosling behavior: the Context Manager never runs.
    #[default]
    Off,
    /// Build the `ContextPacket` and log its metadata, but leave the actual
    /// provider input untouched.
    Shadow,
    /// Build the `ContextPacket` and use it as the provider input.
    On,
}

impl ContextManagerMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ContextManagerMode::Off => "off",
            ContextManagerMode::Shadow => "shadow",
            ContextManagerMode::On => "on",
        }
    }
}

/// Reads `GOSLING_CONTEXT_MANAGER` (env var or config), defaulting to `off`.
pub fn context_manager_mode() -> ContextManagerMode {
    let raw = Config::global()
        .get_param::<String>("GOSLING_CONTEXT_MANAGER")
        .unwrap_or_else(|_| "off".to_string());

    match raw.trim().to_lowercase().as_str() {
        "shadow" => ContextManagerMode::Shadow,
        "on" => ContextManagerMode::On,
        _ => ContextManagerMode::Off,
    }
}

/// How many of the most recent messages (before the required tail) are kept
/// in full as `High` priority before older messages become summarization
/// candidates.
pub const RECENT_MESSAGE_WINDOW: usize = 10;

/// A single tool result above this estimated token count is treated as a
/// "long tool output" summarization candidate even inside the recent window.
pub const LONG_TOOL_OUTPUT_TOKEN_THRESHOLD: usize = 800;

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn defaults_to_off() {
        std::env::remove_var("GOSLING_CONTEXT_MANAGER");
        assert_eq!(context_manager_mode(), ContextManagerMode::Off);
    }

    #[test]
    #[serial]
    fn reads_shadow_and_on() {
        std::env::set_var("GOSLING_CONTEXT_MANAGER", "shadow");
        assert_eq!(context_manager_mode(), ContextManagerMode::Shadow);

        std::env::set_var("GOSLING_CONTEXT_MANAGER", "ON");
        assert_eq!(context_manager_mode(), ContextManagerMode::On);

        std::env::remove_var("GOSLING_CONTEXT_MANAGER");
    }
}
