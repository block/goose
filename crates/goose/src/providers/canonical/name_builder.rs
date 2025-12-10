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

/// Try to map a provider/model pair to a canonical model with fuzzy matching
/// This handles:
/// - Prefixed models (goose-claude-4-5-opus → anthropic/claude-4.5-opus)
/// - Hosting providers (databricks returning claude-3-5-sonnet → anthropic/claude-3.5-sonnet)
/// - Provider-prefixed models (databricks-meta-llama → meta-llama/llama)
/// - Word order variations (claude-4-opus ↔ claude-opus-4)
pub fn fuzzy_canonical_name(provider: &str, model: &str) -> Vec<String> {
    let mut candidates = Vec::new();

    // Always try the standard canonical name first
    candidates.push(canonical_name(provider, model));

    // Strip common prefixes and try again
    let model_stripped = strip_common_prefixes(model);
    if model_stripped != model {
        candidates.push(canonical_name(provider, &model_stripped));
    }

    // Try word-order swapping for Claude models (claude-4-opus ↔ claude-opus-4)
    if let Some(swapped) = swap_claude_word_order(&model_stripped) {
        candidates.push(canonical_name(provider, &swapped));

        // For hosting providers, also try with inferred provider
        if is_hosting_provider(provider) {
            if let Some(inferred) = infer_provider_from_model(&swapped) {
                candidates.push(canonical_name(inferred, &swapped));
            }
        }
    }

    // For hosting providers, try to infer the real provider from model name patterns
    if is_hosting_provider(provider) {
        if let Some(inferred_provider) = infer_provider_from_model(&model_stripped) {
            candidates.push(canonical_name(inferred_provider, &model_stripped));
        }

        // Also try without any provider context
        if let Some(inferred) = infer_provider_from_model(model) {
            candidates.push(canonical_name(inferred, model));
        }
    }

    // For provider-prefixed models like "databricks-meta-llama-3-1-70b"
    // Extract the real provider and model
    if let Some((extracted_provider, extracted_model)) = extract_provider_prefix(&model_stripped) {
        candidates.push(canonical_name(extracted_provider, extracted_model));
    }

    candidates
}

/// Swap word order for Claude models to handle both naming conventions
/// Claude 3: claude-3.5-sonnet ↔ claude-sonnet-3.5
/// Claude 4: claude-4-opus ↔ claude-opus-4
fn swap_claude_word_order(model: &str) -> Option<String> {
    if !model.starts_with("claude-") {
        return None;
    }

    // Pattern: claude-{version}-{size} → claude-{size}-{version}
    // Examples: claude-3-5-sonnet → claude-sonnet-3-5, claude-4-opus → claude-opus-4
    // Handle both dots and dashes in version numbers: 3.5 or 3-5
    let size_patterns = ["sonnet", "opus", "haiku"];

    for size in &size_patterns {
        // Match: claude-{digits/dots/dashes}-{size}
        // Accepts: claude-3.5-sonnet, claude-3-5-sonnet
        let pattern = format!("claude-([0-9.-]+)-{}", size);
        let re = regex::Regex::new(&pattern).unwrap();

        if let Some(captures) = re.captures(model) {
            let version = &captures[1];
            return Some(format!("claude-{}-{}", size, version));
        }

        // Also try reverse: claude-{size}-{digits/dots/dashes}
        // Accepts: claude-sonnet-3.5, claude-haiku-3-5
        let reverse_pattern = format!("claude-{}-([0-9.-]+)", size);
        let re_reverse = regex::Regex::new(&reverse_pattern).unwrap();

        if let Some(captures) = re_reverse.captures(model) {
            let version = &captures[1];
            return Some(format!("claude-{}-{}", version, size));
        }
    }

    None
}

/// Check if provider is a hosting provider that can serve models from other providers
fn is_hosting_provider(provider: &str) -> bool {
    matches!(provider, "databricks" | "openrouter" | "azure" | "bedrock")
}

/// Infer the real provider from model name patterns
fn infer_provider_from_model(model: &str) -> Option<&'static str> {
    let model_lower = model.to_lowercase();

    // Claude models → Anthropic
    if model_lower.starts_with("claude-") || model_lower.contains("claude") {
        return Some("anthropic");
    }

    // GPT, O1, O3, ChatGPT models → OpenAI
    if model_lower.starts_with("gpt-")
        || model_lower.starts_with("o1")
        || model_lower.starts_with("o3")
        || model_lower.starts_with("chatgpt-") {
        return Some("openai");
    }

    // Gemini, Gemma models → Google
    if model_lower.starts_with("gemini-") || model_lower.starts_with("gemma-") {
        return Some("google");
    }

    // Llama models → meta-llama
    if model_lower.contains("llama") {
        return Some("meta-llama");
    }

    // Mistral models → mistralai
    if model_lower.starts_with("mistral-") || model_lower.contains("mixtral") {
        return Some("mistralai");
    }

    None
}

/// Strip common prefixes from model names using pattern matching
/// Looks for known model family patterns and strips everything before them
fn strip_common_prefixes(model: &str) -> String {
    // Known model family patterns (in order of specificity)
    let model_patterns = [
        "meta-llama-",     // Keep meta-llama prefix
        "claude-",
        "gpt-",
        "gemini-",
        "gemma-",
        "o1-",
        "o1",              // Just "o1" without hyphen
        "o3-",
        "o3",
        "o4-",
        "mistral-",
        "mixtral-",
        "chatgpt-",
    ];

    // Find the first occurrence of any known model pattern
    let mut earliest_pos = None;

    for pattern in &model_patterns {
        if let Some(pos) = model.to_lowercase().find(pattern) {
            if earliest_pos.is_none() || pos < earliest_pos.unwrap() {
                earliest_pos = Some(pos);
            }
        }
    }

    // If we found a pattern, strip everything before it
    if let Some(pos) = earliest_pos {
        // Extract from the position of the matched pattern
        return model[pos..].to_string();
    }

    model.to_string()
}

/// Try to extract provider prefix from model names like "databricks-meta-llama-3-1-70b"
/// Returns (provider, model) tuple if found
fn extract_provider_prefix(model: &str) -> Option<(&'static str, &str)> {
    let known_providers = [
        "anthropic",
        "openai",
        "google",
        "meta-llama",
        "mistralai",
        "cohere",
        "ai21",
        "amazon",
    ];

    for provider in &known_providers {
        let prefix = format!("{}-", provider);
        if model.starts_with(&prefix) {
            if let Some(model_part) = model.strip_prefix(&prefix) {
                return Some((provider, model_part));
            }
        }
    }

    None
}

/// Strip version suffixes from model names and normalize version numbers
pub fn strip_version_suffix(model: &str) -> String {
    // First, normalize version numbers: convert -X-Y- to -X.Y- (e.g., -3-5- to -3.5-)
    // This handles cases where Anthropic uses dashes (claude-3-5-haiku) but OpenRouter uses dots (claude-3.5-haiku)
    let normalize_version = regex::Regex::new(r"-(\d)-(\d)(-|$)").unwrap();
    let mut result = normalize_version.replace_all(model, "-$1.$2$3").to_string();

    // Strip datetime, version, and preview/exp suffixes
    let patterns = [
        regex::Regex::new(r"-latest$").unwrap(),         // -latest
        regex::Regex::new(r"-preview(-\d+)*$").unwrap(), // -preview, -preview-09, -preview-05-20
        regex::Regex::new(r"-exp(-\d+)*$").unwrap(),     // -exp, -exp-1219, -exp-01-21
        regex::Regex::new(r":exacto$").unwrap(),         // :exacto (OpenRouter provider suffix)
        regex::Regex::new(r"-\d{8}$").unwrap(),          // -20241022
        regex::Regex::new(r"-\d{4}-\d{2}-\d{2}$").unwrap(), // -2024-04-09
        regex::Regex::new(r"-v\d+(\.\d+)*$").unwrap(),   // -v1.5 (semantic versions with "v" prefix)
        regex::Regex::new(r"-\d{3,}$").unwrap(),         // -002, -001 (patch versions: 3+ digits only)
        regex::Regex::new(r"-bedrock$").unwrap(),        // -bedrock (platform suffixes)
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
        assert_eq!(
            canonical_name("anthropic", "claude-3-5-sonnet-20241022"),
            "anthropic/claude-3.5-sonnet"
        );
        assert_eq!(
            canonical_name("openai", "gpt-4-turbo-2024-04-09"),
            "openai/gpt-4-turbo"
        );
        assert_eq!(
            canonical_name("google", "gemini-1.5-pro-002"),
            "google/gemini-1.5-pro"
        );
        assert_eq!(
            canonical_name("anthropic", "claude-3-5-sonnet"),
            "anthropic/claude-3.5-sonnet"
        );
        assert_eq!(
            canonical_name("openrouter", "anthropic/claude-3.5-sonnet"),
            "anthropic/claude-3.5-sonnet"
        );
        assert_eq!(
            canonical_name("openrouter", "openai/gpt-4-turbo-2024-04-09"),
            "openai/gpt-4-turbo"
        );

        // strip_version_suffix - 8 digit dates
        assert_eq!(
            strip_version_suffix("claude-3-5-sonnet-20241022"),
            "claude-3.5-sonnet"
        );

        // strip_version_suffix - YYYY-MM-DD dates
        assert_eq!(
            strip_version_suffix("gpt-4-turbo-2024-04-09"),
            "gpt-4-turbo"
        );

        // strip_version_suffix - patch versions (3+ digits) and semantic versions
        assert_eq!(strip_version_suffix("gemini-1.5-pro-002"), "gemini-1.5-pro");
        assert_eq!(strip_version_suffix("gemini-1.5-pro-001"), "gemini-1.5-pro");
        assert_eq!(strip_version_suffix("model-v1.5"), "model");
        assert_eq!(strip_version_suffix("model-v2.0"), "model");

        // strip_version_suffix - no suffix
        assert_eq!(
            strip_version_suffix("claude-3-5-sonnet"),
            "claude-3.5-sonnet"
        );

        // strip_version_suffix - exp suffix
        assert_eq!(
            strip_version_suffix("gemini-2.0-flash-exp"),
            "gemini-2.0-flash"
        );
        assert_eq!(
            strip_version_suffix("gemini-2.0-flash-thinking-exp-01-21"),
            "gemini-2.0-flash-thinking"
        );

        // strip_version_suffix - preview suffix
        assert_eq!(
            strip_version_suffix("gemini-2.5-flash-preview"),
            "gemini-2.5-flash"
        );
        assert_eq!(
            strip_version_suffix("gemini-2.5-flash-preview-05-20"),
            "gemini-2.5-flash"
        );
        assert_eq!(
            strip_version_suffix("gemini-2.5-flash-lite-preview-09"),
            "gemini-2.5-flash-lite"
        );

        // strip_version_suffix - multiple patterns
        assert_eq!(
            strip_version_suffix("gemini-2.5-pro-preview-03-25"),
            "gemini-2.5-pro"
        );

        // normalize version numbers (dashes to dots)
        assert_eq!(strip_version_suffix("claude-3-5-haiku"), "claude-3.5-haiku");
        assert_eq!(
            strip_version_suffix("claude-3-7-sonnet"),
            "claude-3.7-sonnet"
        );
        assert_eq!(strip_version_suffix("claude-haiku-4-5"), "claude-haiku-4.5");
        assert_eq!(strip_version_suffix("claude-opus-4-1"), "claude-opus-4.1");
        assert_eq!(
            strip_version_suffix("claude-sonnet-4-5"),
            "claude-sonnet-4.5"
        );
        assert_eq!(strip_version_suffix("claude-sonnet-4"), "claude-sonnet-4");

        // normalize and strip combined
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
            "claude-haiku-4.5"
        );
        assert_eq!(
            strip_version_suffix("claude-sonnet-4-5-20250929"),
            "claude-sonnet-4.5"
        );

        // preserve model family versions (1-2 digits)
        assert_eq!(
            strip_version_suffix("claude-sonnet-4.5"),
            "claude-sonnet-4.5"
        );
        assert_eq!(strip_version_suffix("claude-sonnet-4"), "claude-sonnet-4");
        assert_eq!(strip_version_suffix("claude-haiku-4.5"), "claude-haiku-4.5");
        assert_eq!(strip_version_suffix("gpt-4-turbo"), "gpt-4-turbo");
        assert_eq!(strip_version_suffix("gpt-3.5-turbo"), "gpt-3.5-turbo");
        assert_eq!(strip_version_suffix("model-002"), "model");
        assert_eq!(strip_version_suffix("model-123"), "model");

        // strip -latest suffix
        assert_eq!(
            strip_version_suffix("claude-3.5-sonnet-latest"),
            "claude-3.5-sonnet"
        );
        assert_eq!(strip_version_suffix("gpt-4o-latest"), "gpt-4o");
        assert_eq!(
            strip_version_suffix("chatgpt-4o-latest"),
            "chatgpt-4o"
        );
    }

    #[test]
    fn test_fuzzy_canonical_name() {
        // Test hosting provider with direct model names (Databricks pattern)
        let candidates = fuzzy_canonical_name("databricks", "claude-3-5-sonnet");
        assert!(candidates.contains(&"anthropic/claude-3.5-sonnet".to_string()));

        let candidates = fuzzy_canonical_name("databricks", "gpt-4o");
        assert!(candidates.contains(&"openai/gpt-4o".to_string()));

        let candidates = fuzzy_canonical_name("databricks", "gemini-2-5-flash");
        assert!(candidates.contains(&"google/gemini-2.5-flash".to_string()));

        // Test word-order swapping (Claude 3 series: version-size ↔ size-version)
        let candidates = fuzzy_canonical_name("databricks", "claude-haiku-3-5");
        assert!(candidates.contains(&"anthropic/claude-3.5-haiku".to_string()));

        let candidates = fuzzy_canonical_name("databricks", "claude-sonnet-3-7");
        assert!(candidates.contains(&"anthropic/claude-3.7-sonnet".to_string()));

        // Test word-order swapping (Claude 4 series: version-size ↔ size-version)
        let candidates = fuzzy_canonical_name("databricks", "claude-4-opus");
        assert!(candidates.contains(&"anthropic/claude-opus-4".to_string()));

        let candidates = fuzzy_canonical_name("databricks", "claude-4-sonnet");
        assert!(candidates.contains(&"anthropic/claude-sonnet-4".to_string()));

        // Test prefixed models with word-order swapping
        let candidates = fuzzy_canonical_name("databricks", "goose-claude-4-opus");
        assert!(candidates.contains(&"anthropic/claude-opus-4".to_string()));

        let candidates = fuzzy_canonical_name("databricks", "kgoose-claude-4-sonnet");
        assert!(candidates.contains(&"anthropic/claude-sonnet-4".to_string()));

        let candidates = fuzzy_canonical_name("databricks", "headless-goose-claude-4-sonnet");
        assert!(candidates.contains(&"anthropic/claude-sonnet-4".to_string()));

        let candidates = fuzzy_canonical_name("databricks", "kgoose-cashapp-claude-4-sonnet");
        assert!(candidates.contains(&"anthropic/claude-sonnet-4".to_string()));

        // Test ng-tools prefix with word swapping
        let candidates = fuzzy_canonical_name("databricks", "ng-tools-claude-haiku-3-5");
        assert!(candidates.contains(&"anthropic/claude-3.5-haiku".to_string()));

        // Test raml prefix
        let candidates = fuzzy_canonical_name("databricks", "raml-claude-opus-4-5");
        assert!(candidates.contains(&"anthropic/claude-opus-4.5".to_string()));

        // Test databricks prefix
        let candidates = fuzzy_canonical_name("databricks", "databricks-claude-sonnet-4-5");
        assert!(candidates.contains(&"anthropic/claude-sonnet-4.5".to_string()));

        // Test multiple prefixes (should strip all)
        let candidates = fuzzy_canonical_name("databricks", "kgoose-gpt-4o");
        assert!(candidates.contains(&"openai/gpt-4o".to_string()));

        // Test platform suffixes
        let candidates = fuzzy_canonical_name("databricks", "claude-4-sonnet-bedrock");
        assert!(candidates.contains(&"anthropic/claude-sonnet-4".to_string()));

        let candidates = fuzzy_canonical_name("databricks", "goose-claude-4-sonnet-bedrock");
        assert!(candidates.contains(&"anthropic/claude-sonnet-4".to_string()));

        // Test provider-prefixed models with dates
        let candidates = fuzzy_canonical_name("databricks", "claude-3-5-sonnet-20241022");
        assert!(candidates.contains(&"anthropic/claude-3.5-sonnet".to_string()));

        let candidates = fuzzy_canonical_name("databricks", "gpt-4o-2024-11-20");
        assert!(candidates.contains(&"openai/gpt-4o".to_string()));

        // Test -latest suffix
        let candidates = fuzzy_canonical_name("databricks", "claude-3-5-sonnet-latest");
        assert!(candidates.contains(&"anthropic/claude-3.5-sonnet".to_string()));

        let candidates = fuzzy_canonical_name("databricks", "gpt-4o-latest");
        assert!(candidates.contains(&"openai/gpt-4o".to_string()));

        // Test direct provider (non-hosting)
        let candidates = fuzzy_canonical_name("anthropic", "claude-3-5-sonnet-20241022");
        assert!(candidates.contains(&"anthropic/claude-3.5-sonnet".to_string()));

        let candidates = fuzzy_canonical_name("openai", "gpt-4o-latest");
        assert!(candidates.contains(&"openai/gpt-4o".to_string()));

        // Test O-series models
        let candidates = fuzzy_canonical_name("databricks", "goose-o1");
        assert!(candidates.contains(&"openai/o1".to_string()));

        let candidates = fuzzy_canonical_name("databricks", "kgoose-o3");
        assert!(candidates.contains(&"openai/o3".to_string()));

        let candidates = fuzzy_canonical_name("databricks", "headless-goose-o3-mini");
        assert!(candidates.contains(&"openai/o3-mini".to_string()));
    }

    #[test]
    fn test_infer_provider_from_model() {
        assert_eq!(infer_provider_from_model("claude-3-5-sonnet"), Some("anthropic"));
        assert_eq!(infer_provider_from_model("claude-4-opus"), Some("anthropic"));
        assert_eq!(infer_provider_from_model("gpt-4o"), Some("openai"));
        assert_eq!(infer_provider_from_model("gpt-4-turbo"), Some("openai"));
        assert_eq!(infer_provider_from_model("o1"), Some("openai"));
        assert_eq!(infer_provider_from_model("o3-mini"), Some("openai"));
        assert_eq!(infer_provider_from_model("chatgpt-4o-latest"), Some("openai"));
        assert_eq!(infer_provider_from_model("gemini-2-5-flash"), Some("google"));
        assert_eq!(infer_provider_from_model("gemini-2-5-pro"), Some("google"));
        assert_eq!(infer_provider_from_model("gemma-2-27b-it"), Some("google"));
        assert_eq!(infer_provider_from_model("llama-3-1-70b"), Some("meta-llama"));
        assert_eq!(infer_provider_from_model("mistral-large"), Some("mistralai"));
        assert_eq!(infer_provider_from_model("mixtral-8x7b"), Some("mistralai"));
        assert_eq!(infer_provider_from_model("unknown-model"), None);
    }

    #[test]
    fn test_strip_common_prefixes() {
        assert_eq!(strip_common_prefixes("goose-claude-4-opus"), "claude-4-opus");
        assert_eq!(strip_common_prefixes("databricks-gpt-5"), "gpt-5");
        assert_eq!(strip_common_prefixes("kgoose-gemini-pro"), "gemini-pro");
        assert_eq!(strip_common_prefixes("kgoose-gpt-4o"), "gpt-4o");
        assert_eq!(strip_common_prefixes("azure-gpt-4o"), "gpt-4o");
        assert_eq!(strip_common_prefixes("bedrock-claude-3-5-sonnet"), "claude-3-5-sonnet");
        assert_eq!(strip_common_prefixes("ng-tools-claude-opus-4"), "claude-opus-4");
        assert_eq!(strip_common_prefixes("raml-claude-sonnet-4-5"), "claude-sonnet-4-5"); // version normalization happens later
        assert_eq!(strip_common_prefixes("headless-goose-o3-mini"), "o3-mini");
        assert_eq!(strip_common_prefixes("kgoose-cashapp-claude-4-sonnet"), "claude-4-sonnet");
        assert_eq!(strip_common_prefixes("claude-3-5-sonnet"), "claude-3-5-sonnet"); // no prefix
    }

    #[test]
    fn test_extract_provider_prefix() {
        assert_eq!(extract_provider_prefix("anthropic-claude-3-5-sonnet"), Some(("anthropic", "claude-3-5-sonnet")));
        assert_eq!(extract_provider_prefix("openai-gpt-4o"), Some(("openai", "gpt-4o")));
        assert_eq!(extract_provider_prefix("google-gemini-2-5-flash"), Some(("google", "gemini-2-5-flash")));
        assert_eq!(extract_provider_prefix("meta-llama-3-1-70b"), Some(("meta-llama", "3-1-70b")));
        assert_eq!(extract_provider_prefix("mistralai-mistral-large"), Some(("mistralai", "mistral-large")));
        assert_eq!(extract_provider_prefix("claude-3-5-sonnet"), None); // no provider prefix
        assert_eq!(extract_provider_prefix("unknown-provider-model"), None); // unknown provider
    }

    #[test]
    fn test_is_hosting_provider() {
        assert!(is_hosting_provider("databricks"));
        assert!(is_hosting_provider("openrouter"));
        assert!(is_hosting_provider("azure"));
        assert!(is_hosting_provider("bedrock"));
        assert!(!is_hosting_provider("anthropic"));
        assert!(!is_hosting_provider("openai"));
        assert!(!is_hosting_provider("google"));
    }

    #[test]
    fn test_swap_claude_word_order() {
        // Claude 3 series: version-size to size-version (with dots)
        assert_eq!(swap_claude_word_order("claude-3.5-sonnet"), Some("claude-sonnet-3.5".to_string()));
        assert_eq!(swap_claude_word_order("claude-3.5-haiku"), Some("claude-haiku-3.5".to_string()));
        assert_eq!(swap_claude_word_order("claude-3.7-sonnet"), Some("claude-sonnet-3.7".to_string()));

        // Claude 3 series with dashes in version (before normalization)
        assert_eq!(swap_claude_word_order("claude-3-5-sonnet"), Some("claude-sonnet-3-5".to_string()));
        assert_eq!(swap_claude_word_order("claude-3-7-sonnet"), Some("claude-sonnet-3-7".to_string()));

        // Reverse: size-version to version-size (with dashes)
        assert_eq!(swap_claude_word_order("claude-haiku-3-5"), Some("claude-3-5-haiku".to_string()));
        assert_eq!(swap_claude_word_order("claude-sonnet-3-7"), Some("claude-3-7-sonnet".to_string()));

        // Claude 4 series: size-version to version-size
        assert_eq!(swap_claude_word_order("claude-opus-4"), Some("claude-4-opus".to_string()));
        assert_eq!(swap_claude_word_order("claude-sonnet-4"), Some("claude-4-sonnet".to_string()));
        assert_eq!(swap_claude_word_order("claude-haiku-4.5"), Some("claude-4.5-haiku".to_string()));
        assert_eq!(swap_claude_word_order("claude-sonnet-4.5"), Some("claude-4.5-sonnet".to_string()));

        // Claude 4 series reverse: version-size to size-version
        assert_eq!(swap_claude_word_order("claude-4-opus"), Some("claude-opus-4".to_string()));
        assert_eq!(swap_claude_word_order("claude-4-sonnet"), Some("claude-sonnet-4".to_string()));

        // Non-claude models should return None
        assert_eq!(swap_claude_word_order("gpt-4o"), None);
        assert_eq!(swap_claude_word_order("gemini-2.5-flash"), None);
    }

    #[test]
    fn test_strip_version_suffix_special_cases() {
        // Test -bedrock suffix
        assert_eq!(strip_version_suffix("claude-4-sonnet-bedrock"), "claude-4-sonnet");

        // Ensure we don't strip the main version number
        assert_eq!(strip_version_suffix("claude-4"), "claude-4");
        assert_eq!(strip_version_suffix("gpt-4"), "gpt-4");
    }
}
