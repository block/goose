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

/// Strip version suffixes from model names and normalize version numbers
///
/// First normalizes version numbers (e.g., `-3-5-` → `-3.5-`), then strips:
/// - `-20241022` (8-digit dates)
/// - `-2024-04-09` (YYYY-MM-DD dates)
/// - `-002` or `-v1.5` (version numbers)
/// - `-exp`, `-exp-1219`, `-exp-01-21` (experimental suffixes)
/// - `-preview`, `-preview-09`, `-preview-05-20` (preview suffixes)
///
/// Examples:
/// - `strip_version_suffix("claude-3-5-sonnet-20241022")` → `"claude-3.5-sonnet"`
/// - `strip_version_suffix("gpt-4-turbo-2024-04-09")` → `"gpt-4-turbo"`
/// - `strip_version_suffix("gemini-1.5-pro-002")` → `"gemini-1.5-pro"`
/// - `strip_version_suffix("gemini-2.0-flash-exp")` → `"gemini-2.0-flash"`
/// - `strip_version_suffix("gemini-2.5-flash-preview-05-20")` → `"gemini-2.5-flash"`
pub fn strip_version_suffix(model: &str) -> String {
    // First, normalize version numbers: convert -X-Y- to -X.Y- (e.g., -3-5- to -3.5-)
    // This handles cases where Anthropic uses dashes (claude-3-5-haiku) but OpenRouter uses dots (claude-3.5-haiku)
    let normalize_version = regex::Regex::new(r"-(\d)-(\d)(-|$)").unwrap();
    let mut result = normalize_version.replace_all(model, "-$1.$2$3").to_string();

    // Strip datetime, version, and preview/exp suffixes
    let patterns = [
        regex::Regex::new(r"-preview(-\d+)*$").unwrap(),  // -preview, -preview-09, -preview-05-20
        regex::Regex::new(r"-exp(-\d+)*$").unwrap(),      // -exp, -exp-1219, -exp-01-21
        regex::Regex::new(r"-\d{8}$").unwrap(),           // -20241022
        regex::Regex::new(r"-\d{4}-\d{2}-\d{2}$").unwrap(), // -2024-04-09
        regex::Regex::new(r"-v?\d+(\.\d+)*$").unwrap(),   // -v1.5 or -002
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
    fn test_canonical_name_anthropic() {
        assert_eq!(
            canonical_name("anthropic", "claude-3-5-sonnet-20241022"),
            "anthropic/claude-3.5-sonnet"
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
            "anthropic/claude-3.5-sonnet"
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
            "claude-3.5-sonnet"
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
        assert_eq!(strip_version_suffix("claude-3-5-sonnet"), "claude-3.5-sonnet");
    }

    #[test]
    fn test_strip_version_suffix_exp() {
        assert_eq!(strip_version_suffix("gemini-2.0-flash-exp"), "gemini-2.0-flash");
        assert_eq!(
            strip_version_suffix("gemini-2.0-flash-thinking-exp-01-21"),
            "gemini-2.0-flash-thinking"
        );
    }

    #[test]
    fn test_strip_version_suffix_preview() {
        assert_eq!(strip_version_suffix("gemini-2.5-flash-preview"), "gemini-2.5-flash");
        assert_eq!(
            strip_version_suffix("gemini-2.5-flash-preview-05-20"),
            "gemini-2.5-flash"
        );
        assert_eq!(
            strip_version_suffix("gemini-2.5-flash-lite-preview-09"),
            "gemini-2.5-flash-lite"
        );
    }

    #[test]
    fn test_strip_version_suffix_multiple_patterns() {
        // Should handle preview followed by date
        assert_eq!(
            strip_version_suffix("gemini-2.5-pro-preview-03-25"),
            "gemini-2.5-pro"
        );
    }

    #[test]
    fn test_normalize_version_numbers() {
        // Anthropic models use dashes (3-5) while OpenRouter uses dots (3.5)
        assert_eq!(strip_version_suffix("claude-3-5-haiku"), "claude-3.5-haiku");
        assert_eq!(strip_version_suffix("claude-3-7-sonnet"), "claude-3.7-sonnet");
        // When model name ends with version like "claude-haiku-4-5", it normalizes to 4.5 then strips it
        assert_eq!(strip_version_suffix("claude-haiku-4-5"), "claude-haiku");
        assert_eq!(strip_version_suffix("claude-opus-4-1"), "claude-opus");
    }

    #[test]
    fn test_normalize_and_strip() {
        // Should normalize version numbers AND strip date suffixes
        assert_eq!(
            strip_version_suffix("claude-3-5-haiku-20241022"),
            "claude-3.5-haiku"
        );
        assert_eq!(
            strip_version_suffix("claude-3-7-sonnet-20250219"),
            "claude-3.7-sonnet"
        );
        assert_eq!(
            strip_version_suffix("claude-haiku-4-5-20251001"),
            "claude-haiku"
        );
        assert_eq!(
            strip_version_suffix("claude-sonnet-4-5-20250929"),
            "claude-sonnet"
        );
    }
}
