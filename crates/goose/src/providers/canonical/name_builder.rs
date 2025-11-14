/// Build canonical model name from provider and model identifiers
pub fn canonical_name(provider: &str, model: &str) -> String {
    let model_base = strip_version_suffix(model);

    // OpenRouter models are already in canonical format
    if provider == "openrouter" {
        model_base
    } else {
        format!("{}/{}", provider, model_base)
    }
}

/// Strip version suffixes from model names and normalize version numbers
pub fn strip_version_suffix(model: &str) -> String {
    // First, normalize version numbers: convert -X-Y- to -X.Y- (e.g., -3-5- to -3.5-)
    // This handles cases where Anthropic uses dashes (claude-3-5-haiku) but OpenRouter uses dots (claude-3.5-haiku)
    let normalize_version = regex::Regex::new(r"-(\d)-(\d)(-|$)").unwrap();
    let mut result = normalize_version.replace_all(model, "-$1.$2$3").to_string();

    // Strip datetime, version, and preview/exp suffixes
    let patterns = [
        regex::Regex::new(r"-preview(-\d+)*$").unwrap(),  // -preview, -preview-09, -preview-05-20
        regex::Regex::new(r"-exp(-\d+)*$").unwrap(),      // -exp, -exp-1219, -exp-01-21
        regex::Regex::new(r":exacto$").unwrap(),          // :exacto (OpenRouter provider suffix)
        regex::Regex::new(r"-\d{8}$").unwrap(),           // -20241022
        regex::Regex::new(r"-\d{4}-\d{2}-\d{2}$").unwrap(), // -2024-04-09
        regex::Regex::new(r"-v\d+(\.\d+)*$").unwrap(),    // -v1.5 (semantic versions with "v" prefix)
        regex::Regex::new(r"-\d{3,}$").unwrap(),          // -002, -001 (patch versions: 3+ digits only)
    ];

    // Apply patterns multiple times to handle cases like "-preview-09-2025"
    let mut changed = true;
    while changed {
        let before = result.clone();
        for pattern in &patterns {
            result = pattern.replace(&result, "").to_string();
        }
        changed = result != before;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canonical_names() {
        // canonical_name tests
        assert_eq!(canonical_name("anthropic", "claude-3-5-sonnet-20241022"), "anthropic/claude-3.5-sonnet");
        assert_eq!(canonical_name("openai", "gpt-4-turbo-2024-04-09"), "openai/gpt-4-turbo");
        assert_eq!(canonical_name("google", "gemini-1.5-pro-002"), "google/gemini-1.5-pro");
        assert_eq!(canonical_name("anthropic", "claude-3-5-sonnet"), "anthropic/claude-3.5-sonnet");
        assert_eq!(canonical_name("openrouter", "anthropic/claude-3.5-sonnet"), "anthropic/claude-3.5-sonnet");
        assert_eq!(canonical_name("openrouter", "openai/gpt-4-turbo-2024-04-09"), "openai/gpt-4-turbo");

        // strip_version_suffix - 8 digit dates
        assert_eq!(strip_version_suffix("claude-3-5-sonnet-20241022"), "claude-3.5-sonnet");

        // strip_version_suffix - YYYY-MM-DD dates
        assert_eq!(strip_version_suffix("gpt-4-turbo-2024-04-09"), "gpt-4-turbo");

        // strip_version_suffix - patch versions (3+ digits) and semantic versions
        assert_eq!(strip_version_suffix("gemini-1.5-pro-002"), "gemini-1.5-pro");
        assert_eq!(strip_version_suffix("gemini-1.5-pro-001"), "gemini-1.5-pro");
        assert_eq!(strip_version_suffix("model-v1.5"), "model");
        assert_eq!(strip_version_suffix("model-v2.0"), "model");

        // strip_version_suffix - no suffix
        assert_eq!(strip_version_suffix("claude-3-5-sonnet"), "claude-3.5-sonnet");

        // strip_version_suffix - exp suffix
        assert_eq!(strip_version_suffix("gemini-2.0-flash-exp"), "gemini-2.0-flash");
        assert_eq!(strip_version_suffix("gemini-2.0-flash-thinking-exp-01-21"), "gemini-2.0-flash-thinking");

        // strip_version_suffix - preview suffix
        assert_eq!(strip_version_suffix("gemini-2.5-flash-preview"), "gemini-2.5-flash");
        assert_eq!(strip_version_suffix("gemini-2.5-flash-preview-05-20"), "gemini-2.5-flash");
        assert_eq!(strip_version_suffix("gemini-2.5-flash-lite-preview-09"), "gemini-2.5-flash-lite");

        // strip_version_suffix - multiple patterns
        assert_eq!(strip_version_suffix("gemini-2.5-pro-preview-03-25"), "gemini-2.5-pro");

        // normalize version numbers (dashes to dots)
        assert_eq!(strip_version_suffix("claude-3-5-haiku"), "claude-3.5-haiku");
        assert_eq!(strip_version_suffix("claude-3-7-sonnet"), "claude-3.7-sonnet");
        assert_eq!(strip_version_suffix("claude-haiku-4-5"), "claude-haiku-4.5");
        assert_eq!(strip_version_suffix("claude-opus-4-1"), "claude-opus-4.1");
        assert_eq!(strip_version_suffix("claude-sonnet-4-5"), "claude-sonnet-4.5");
        assert_eq!(strip_version_suffix("claude-sonnet-4"), "claude-sonnet-4");

        // normalize and strip combined
        assert_eq!(strip_version_suffix("claude-3-5-haiku-20241022"), "claude-3.5-haiku");
        assert_eq!(strip_version_suffix("claude-3-7-sonnet-20250219"), "claude-3.7-sonnet");
        assert_eq!(strip_version_suffix("claude-haiku-4-5-20251001"), "claude-haiku-4.5");
        assert_eq!(strip_version_suffix("claude-sonnet-4-5-20250929"), "claude-sonnet-4.5");

        // preserve model family versions (1-2 digits)
        assert_eq!(strip_version_suffix("claude-sonnet-4.5"), "claude-sonnet-4.5");
        assert_eq!(strip_version_suffix("claude-sonnet-4"), "claude-sonnet-4");
        assert_eq!(strip_version_suffix("claude-haiku-4.5"), "claude-haiku-4.5");
        assert_eq!(strip_version_suffix("gpt-4-turbo"), "gpt-4-turbo");
        assert_eq!(strip_version_suffix("gpt-3.5-turbo"), "gpt-3.5-turbo");
        assert_eq!(strip_version_suffix("model-002"), "model");
        assert_eq!(strip_version_suffix("model-123"), "model");
    }
}
