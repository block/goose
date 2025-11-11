/// Build canonical model name from provider and model identifiers
///
/// This function handles different provider conventions:
/// - **OpenRouter**: Models are already in canonical format (e.g., "anthropic/claude-3.5-sonnet"),
///   so just strip version suffixes
/// - **Other providers** (Anthropic, OpenAI, Google, etc.): Prefix with provider name and strip
///   version suffixes
///
/// Examples:
/// - `canonical_name("anthropic", "claude-3-5-sonnet-20241022")` → `"anthropic/claude-3-5-sonnet"`
/// - `canonical_name("openai", "gpt-4-turbo-2024-04-09")` → `"openai/gpt-4-turbo"`
/// - `canonical_name("google", "gemini-1.5-pro-002")` → `"google/gemini-1.5-pro"`
/// - `canonical_name("openrouter", "anthropic/claude-3.5-sonnet")` → `"anthropic/claude-3.5-sonnet"`
pub fn canonical_name(provider: &str, model: &str) -> String {
    let model_base = strip_version_suffix(model);

    // OpenRouter models are already in canonical format
    if provider == "openrouter" {
        model_base
    } else {
        format!("{}/{}", provider, model_base)
    }
}

/// Strip version suffixes from model names
///
/// Strips patterns like:
/// - `-20241022` (8-digit dates)
/// - `-2024-04-09` (YYYY-MM-DD dates)
/// - `-002` or `-v1.5` (version numbers)
///
/// Examples:
/// - `strip_version_suffix("claude-3-5-sonnet-20241022")` → `"claude-3-5-sonnet"`
/// - `strip_version_suffix("gpt-4-turbo-2024-04-09")` → `"gpt-4-turbo"`
/// - `strip_version_suffix("gemini-1.5-pro-002")` → `"gemini-1.5-pro"`
pub fn strip_version_suffix(model: &str) -> String {
    // Strip datetime suffixes like:
    // - "-20241022" (8 digits)
    // - "-2024-04-09" (YYYY-MM-DD)
    // - "-v1.5" or "-002" (version numbers)
    let patterns = [
        regex::Regex::new(r"-\d{8}$").unwrap(),           // -20241022
        regex::Regex::new(r"-\d{4}-\d{2}-\d{2}$").unwrap(), // -2024-04-09
        regex::Regex::new(r"-v?\d+(\.\d+)*$").unwrap(),   // -v1.5 or -002
    ];

    let mut result = model.to_string();
    for pattern in &patterns {
        result = pattern.replace(&result, "").to_string();
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canonical_name_anthropic() {
        assert_eq!(
            canonical_name("anthropic", "claude-3-5-sonnet-20241022"),
            "anthropic/claude-3-5-sonnet"
        );
    }

    #[test]
    fn test_canonical_name_openai() {
        assert_eq!(
            canonical_name("openai", "gpt-4-turbo-2024-04-09"),
            "openai/gpt-4-turbo"
        );
    }

    #[test]
    fn test_canonical_name_google() {
        assert_eq!(
            canonical_name("google", "gemini-1.5-pro-002"),
            "google/gemini-1.5-pro"
        );
    }

    #[test]
    fn test_canonical_name_no_version() {
        assert_eq!(
            canonical_name("anthropic", "claude-3-5-sonnet"),
            "anthropic/claude-3-5-sonnet"
        );
    }

    #[test]
    fn test_canonical_name_openrouter() {
        assert_eq!(
            canonical_name("openrouter", "anthropic/claude-3.5-sonnet"),
            "anthropic/claude-3.5-sonnet"
        );
    }

    #[test]
    fn test_canonical_name_openrouter_with_version() {
        assert_eq!(
            canonical_name("openrouter", "openai/gpt-4-turbo-2024-04-09"),
            "openai/gpt-4-turbo"
        );
    }

    #[test]
    fn test_strip_version_suffix_8_digit() {
        assert_eq!(
            strip_version_suffix("claude-3-5-sonnet-20241022"),
            "claude-3-5-sonnet"
        );
    }

    #[test]
    fn test_strip_version_suffix_date() {
        assert_eq!(
            strip_version_suffix("gpt-4-turbo-2024-04-09"),
            "gpt-4-turbo"
        );
    }

    #[test]
    fn test_strip_version_suffix_version_number() {
        assert_eq!(strip_version_suffix("gemini-1.5-pro-002"), "gemini-1.5-pro");
        assert_eq!(strip_version_suffix("model-v1.5"), "model");
    }

    #[test]
    fn test_strip_version_suffix_no_suffix() {
        assert_eq!(strip_version_suffix("claude-3-5-sonnet"), "claude-3-5-sonnet");
    }
}
