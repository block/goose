use once_cell::sync::Lazy;
use regex::Regex;

static NORMALIZE_VERSION_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"-(\d)-(\d)(-|$)").expect("Failed to compile NORMALIZE_VERSION regex")
});

static STRIP_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    vec![
        Regex::new(r"-latest$").expect("Failed to compile -latest regex"),
        Regex::new(r"-preview(-\d+)*$").expect("Failed to compile -preview regex"),
        Regex::new(r"-exp(-\d+)*$").expect("Failed to compile -exp regex"),
        Regex::new(r":exacto$").expect("Failed to compile :exacto regex"),
        Regex::new(r"-\d{8}$").expect("Failed to compile date regex"),
        Regex::new(r"-\d{4}-\d{2}-\d{2}$").expect("Failed to compile full-date regex"),
        Regex::new(r"-v\d+(\.\d+)*$").expect("Failed to compile version regex"),
        Regex::new(r"-\d{3,}$").expect("Failed to compile patch regex"),
        Regex::new(r"-bedrock$").expect("Failed to compile -bedrock regex"),
    ]
});

static CLAUDE_PATTERNS: Lazy<Vec<(Regex, Regex, &'static str)>> = Lazy::new(|| {
    ["sonnet", "opus", "haiku"]
        .iter()
        .map(|&size| {
            (
                Regex::new(&format!("claude-([0-9.-]+)-{}", size))
                    .expect("Failed to compile Claude forward regex"),
                Regex::new(&format!("claude-{}-([0-9.-]+)", size))
                    .expect("Failed to compile Claude reverse regex"),
                size,
            )
        })
        .collect()
});

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

/// Try to map a provider/model pair to a canonical model
pub fn map_to_canonical_model(
    provider: &str,
    model: &str,
    registry: &super::CanonicalModelRegistry,
) -> Option<String> {
    // Try direct mapping first
    let candidate = canonical_name(provider, model);
    if registry.get(&candidate).is_some() {
        return Some(candidate);
    }

    // Try with common prefixes stripped
    let model_stripped = strip_common_prefixes(model);
    if model_stripped != model {
        let candidate = canonical_name(provider, &model_stripped);
        if registry.get(&candidate).is_some() {
            return Some(candidate);
        }
    }

    // Try word-order swapping for Claude models (claude-4-opus ↔ claude-opus-4)
    if let Some(swapped) = swap_claude_word_order(&model_stripped) {
        let candidate = canonical_name(provider, &swapped);
        if registry.get(&candidate).is_some() {
            return Some(candidate);
        }

        if is_hosting_provider(provider) {
            if let Some(inferred) = infer_provider_from_model(&swapped) {
                let candidate = canonical_name(inferred, &swapped);
                if registry.get(&candidate).is_some() {
                    return Some(candidate);
                }
            }
        }
    }

    // For hosting providers, try to infer the real provider from model name patterns
    if is_hosting_provider(provider) {
        if let Some(inferred_provider) = infer_provider_from_model(&model_stripped) {
            let candidate = canonical_name(inferred_provider, &model_stripped);
            if registry.get(&candidate).is_some() {
                return Some(candidate);
            }
        }

        if let Some(inferred) = infer_provider_from_model(model) {
            let candidate = canonical_name(inferred, model);
            if registry.get(&candidate).is_some() {
                return Some(candidate);
            }
        }
    }

    // For provider-prefixed models like "databricks-meta-llama-3-1-70b"
    if let Some((extracted_provider, extracted_model)) = extract_provider_prefix(&model_stripped) {
        let candidate = canonical_name(extracted_provider, extracted_model);
        if registry.get(&candidate).is_some() {
            return Some(candidate);
        }
    }

    None
}

/// Swap word order for Claude models to handle both naming conventions
fn swap_claude_word_order(model: &str) -> Option<String> {
    if !model.starts_with("claude-") {
        return None;
    }

    for (forward_re, reverse_re, size) in CLAUDE_PATTERNS.iter() {
        if let Some(captures) = forward_re.captures(model) {
            let version = &captures[1];
            return Some(format!("claude-{}-{}", size, version));
        }

        if let Some(captures) = reverse_re.captures(model) {
            let version = &captures[1];
            return Some(format!("claude-{}-{}", version, size));
        }
    }

    None
}

fn is_hosting_provider(provider: &str) -> bool {
    matches!(provider, "databricks" | "openrouter" | "azure" | "bedrock")
}

/// Infer the real provider from model name patterns
fn infer_provider_from_model(model: &str) -> Option<&'static str> {
    let model_lower = model.to_lowercase();

    if model_lower.starts_with("claude-") || model_lower.contains("claude") {
        return Some("anthropic");
    }

    if model_lower.starts_with("gpt-")
        || model_lower.starts_with("o1")
        || model_lower.starts_with("o3")
        || model_lower.starts_with("o4")
        || model_lower.starts_with("chatgpt-")
    {
        return Some("openai");
    }

    if model_lower.starts_with("gemini-") || model_lower.starts_with("gemma-") {
        return Some("google");
    }

    if model_lower.contains("llama") {
        return Some("meta-llama");
    }

    if model_lower.starts_with("mistral")
        || model_lower.starts_with("mixtral")
        || model_lower.starts_with("codestral")
        || model_lower.starts_with("ministral")
        || model_lower.starts_with("pixtral")
        || model_lower.starts_with("devstral")
        || model_lower.starts_with("voxtral")
    {
        return Some("mistralai");
    }

    if model_lower.starts_with("deepseek") || model_lower.contains("deepseek") {
        return Some("deepseek");
    }

    if model_lower.starts_with("qwen") || model_lower.contains("qwen") {
        return Some("qwen");
    }

    if model_lower.starts_with("grok") || model_lower.contains("grok") {
        return Some("x-ai");
    }

    if model_lower.starts_with("jamba") || model_lower.contains("jamba") {
        return Some("ai21");
    }

    if model_lower.starts_with("command") || model_lower.contains("command") {
        return Some("cohere");
    }

    None
}

/// Strip common prefixes from model names using pattern matching
/// Looks for known model family patterns and strips everything before them
fn strip_common_prefixes(model: &str) -> String {
    let model_patterns = [
        "claude-",
        "gpt-",
        "gemini-",
        "gemma-",
        "o1-",
        "o1",
        "o3-",
        "o3",
        "o4-",
        "meta-llama-",
        "mistral-",
        "mixtral-",
        "chatgpt-",
        "deepseek-",
        "qwen-",
        "grok-",
        "jamba-",
        "command-",
        "codestral",
        "ministral-",
        "pixtral-",
        "devstral-",
    ];

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
        "deepseek",
        "qwen",
        "x-ai",
        "nvidia",
        "microsoft",
        "perplexity",
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
    let mut result = NORMALIZE_VERSION_RE
        .replace_all(model, "-$1.$2$3")
        .to_string();

    let mut changed = true;
    while changed {
        let before = result.clone();
        for pattern in STRIP_PATTERNS.iter() {
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
        assert_eq!(strip_version_suffix("chatgpt-4o-latest"), "chatgpt-4o");
    }

    #[test]
    fn test_fuzzy_canonical_name() {
        let registry = super::super::CanonicalModelRegistry::bundled().unwrap();

        // Test hosting provider with direct model names (Databricks pattern)
        assert_eq!(
            map_to_canonical_model("databricks", "claude-3-5-sonnet", registry),
            Some("anthropic/claude-3.5-sonnet".to_string())
        );

        assert_eq!(
            map_to_canonical_model("databricks", "gpt-4o", registry),
            Some("openai/gpt-4o".to_string())
        );

        assert_eq!(
            map_to_canonical_model("databricks", "gemini-2-5-flash", registry),
            Some("google/gemini-2.5-flash".to_string())
        );

        // Test word-order swapping (Claude 3 series: version-size ↔ size-version)
        assert_eq!(
            map_to_canonical_model("databricks", "claude-haiku-3-5", registry),
            Some("anthropic/claude-3.5-haiku".to_string())
        );

        assert_eq!(
            map_to_canonical_model("databricks", "claude-sonnet-3-7", registry),
            Some("anthropic/claude-3.7-sonnet".to_string())
        );

        // Test word-order swapping (Claude 4 series: version-size ↔ size-version)
        assert_eq!(
            map_to_canonical_model("databricks", "claude-4-opus", registry),
            Some("anthropic/claude-opus-4".to_string())
        );

        assert_eq!(
            map_to_canonical_model("databricks", "claude-4-sonnet", registry),
            Some("anthropic/claude-sonnet-4".to_string())
        );

        // Test prefixed models with word-order swapping
        assert_eq!(
            map_to_canonical_model("databricks", "goose-claude-4-opus", registry),
            Some("anthropic/claude-opus-4".to_string())
        );

        assert_eq!(
            map_to_canonical_model("databricks", "kgoose-claude-4-sonnet", registry),
            Some("anthropic/claude-sonnet-4".to_string())
        );

        assert_eq!(
            map_to_canonical_model("databricks", "headless-goose-claude-4-sonnet", registry),
            Some("anthropic/claude-sonnet-4".to_string())
        );

        assert_eq!(
            map_to_canonical_model("databricks", "kgoose-cashapp-claude-4-sonnet", registry),
            Some("anthropic/claude-sonnet-4".to_string())
        );

        // Test ng-tools prefix with word swapping
        assert_eq!(
            map_to_canonical_model("databricks", "ng-tools-claude-haiku-3-5", registry),
            Some("anthropic/claude-3.5-haiku".to_string())
        );

        // Test raml prefix
        assert_eq!(
            map_to_canonical_model("databricks", "raml-claude-opus-4-5", registry),
            Some("anthropic/claude-opus-4.5".to_string())
        );

        // Test databricks prefix
        assert_eq!(
            map_to_canonical_model("databricks", "databricks-claude-sonnet-4-5", registry),
            Some("anthropic/claude-sonnet-4.5".to_string())
        );

        // Test multiple prefixes (should strip all)
        assert_eq!(
            map_to_canonical_model("databricks", "kgoose-gpt-4o", registry),
            Some("openai/gpt-4o".to_string())
        );

        // Test platform suffixes
        assert_eq!(
            map_to_canonical_model("databricks", "claude-4-sonnet-bedrock", registry),
            Some("anthropic/claude-sonnet-4".to_string())
        );

        assert_eq!(
            map_to_canonical_model("databricks", "goose-claude-4-sonnet-bedrock", registry),
            Some("anthropic/claude-sonnet-4".to_string())
        );

        // Test provider-prefixed models with dates
        assert_eq!(
            map_to_canonical_model("databricks", "claude-3-5-sonnet-20241022", registry),
            Some("anthropic/claude-3.5-sonnet".to_string())
        );

        assert_eq!(
            map_to_canonical_model("databricks", "gpt-4o-2024-11-20", registry),
            Some("openai/gpt-4o".to_string())
        );

        // Test -latest suffix
        assert_eq!(
            map_to_canonical_model("databricks", "claude-3-5-sonnet-latest", registry),
            Some("anthropic/claude-3.5-sonnet".to_string())
        );

        assert_eq!(
            map_to_canonical_model("databricks", "gpt-4o-latest", registry),
            Some("openai/gpt-4o".to_string())
        );

        // Test direct provider (non-hosting)
        assert_eq!(
            map_to_canonical_model("anthropic", "claude-3-5-sonnet-20241022", registry),
            Some("anthropic/claude-3.5-sonnet".to_string())
        );

        assert_eq!(
            map_to_canonical_model("openai", "gpt-4o-latest", registry),
            Some("openai/gpt-4o".to_string())
        );

        // Test O-series models
        assert_eq!(
            map_to_canonical_model("databricks", "goose-o1", registry),
            Some("openai/o1".to_string())
        );

        assert_eq!(
            map_to_canonical_model("databricks", "kgoose-o3", registry),
            Some("openai/o3".to_string())
        );

        assert_eq!(
            map_to_canonical_model("databricks", "headless-goose-o3-mini", registry),
            Some("openai/o3-mini".to_string())
        );

        // Test new providers: DeepSeek
        assert_eq!(
            map_to_canonical_model("databricks", "databricks-deepseek-chat", registry),
            Some("deepseek/deepseek-chat".to_string())
        );

        assert_eq!(
            map_to_canonical_model("databricks", "deepseek-r1", registry),
            Some("deepseek/deepseek-r1".to_string())
        );

        // Test Qwen models
        assert_eq!(
            map_to_canonical_model("databricks", "qwen-2-5-72b-instruct", registry),
            Some("qwen/qwen-2.5-72b-instruct".to_string())
        );

        // Test Grok models
        assert_eq!(
            map_to_canonical_model("databricks", "grok-3", registry),
            Some("x-ai/grok-3".to_string())
        );

        assert_eq!(
            map_to_canonical_model("databricks", "databricks-grok-4-fast", registry),
            Some("x-ai/grok-4-fast".to_string())
        );

        // Test Jamba models
        assert_eq!(
            map_to_canonical_model("databricks", "jamba-large-1-7", registry),
            Some("ai21/jamba-large-1.7".to_string())
        );

        // Test Cohere Command models
        assert_eq!(
            map_to_canonical_model("databricks", "command-r-plus-08", registry),
            Some("cohere/command-r-plus-08".to_string())
        );

        // Test Mistral variants
        assert_eq!(
            map_to_canonical_model("databricks", "codestral", registry),
            Some("mistralai/codestral".to_string())
        );

        assert_eq!(
            map_to_canonical_model("databricks", "ministral-8b", registry),
            Some("mistralai/ministral-8b".to_string())
        );
    }

    #[test]
    fn test_infer_provider_from_model() {
        assert_eq!(
            infer_provider_from_model("claude-3-5-sonnet"),
            Some("anthropic")
        );
        assert_eq!(
            infer_provider_from_model("claude-4-opus"),
            Some("anthropic")
        );
        assert_eq!(infer_provider_from_model("gpt-4o"), Some("openai"));
        assert_eq!(infer_provider_from_model("gpt-4-turbo"), Some("openai"));
        assert_eq!(infer_provider_from_model("o1"), Some("openai"));
        assert_eq!(infer_provider_from_model("o3-mini"), Some("openai"));
        assert_eq!(
            infer_provider_from_model("chatgpt-4o-latest"),
            Some("openai")
        );
        assert_eq!(
            infer_provider_from_model("gemini-2-5-flash"),
            Some("google")
        );
        assert_eq!(infer_provider_from_model("gemini-2-5-pro"), Some("google"));
        assert_eq!(infer_provider_from_model("gemma-2-27b-it"), Some("google"));
        assert_eq!(
            infer_provider_from_model("llama-3-1-70b"),
            Some("meta-llama")
        );
        assert_eq!(
            infer_provider_from_model("mistral-large"),
            Some("mistralai")
        );
        assert_eq!(infer_provider_from_model("mixtral-8x7b"), Some("mistralai"));
        assert_eq!(infer_provider_from_model("codestral"), Some("mistralai"));
        assert_eq!(infer_provider_from_model("ministral-8b"), Some("mistralai"));
        assert_eq!(
            infer_provider_from_model("pixtral-large"),
            Some("mistralai")
        );
        assert_eq!(infer_provider_from_model("deepseek-chat"), Some("deepseek"));
        assert_eq!(infer_provider_from_model("deepseek-r1"), Some("deepseek"));
        assert_eq!(
            infer_provider_from_model("qwen-2-5-72b-instruct"),
            Some("qwen")
        );
        assert_eq!(infer_provider_from_model("grok-3"), Some("x-ai"));
        assert_eq!(infer_provider_from_model("grok-4-fast"), Some("x-ai"));
        assert_eq!(infer_provider_from_model("jamba-large-1-7"), Some("ai21"));
        assert_eq!(
            infer_provider_from_model("command-r-plus-08"),
            Some("cohere")
        );
        assert_eq!(infer_provider_from_model("unknown-model"), None);
    }

    #[test]
    fn test_strip_common_prefixes() {
        assert_eq!(
            strip_common_prefixes("goose-claude-4-opus"),
            "claude-4-opus"
        );
        assert_eq!(strip_common_prefixes("databricks-gpt-5"), "gpt-5");
        assert_eq!(strip_common_prefixes("kgoose-gemini-pro"), "gemini-pro");
        assert_eq!(strip_common_prefixes("kgoose-gpt-4o"), "gpt-4o");
        assert_eq!(strip_common_prefixes("azure-gpt-4o"), "gpt-4o");
        assert_eq!(
            strip_common_prefixes("bedrock-claude-3-5-sonnet"),
            "claude-3-5-sonnet"
        );
        assert_eq!(
            strip_common_prefixes("ng-tools-claude-opus-4"),
            "claude-opus-4"
        );
        assert_eq!(
            strip_common_prefixes("raml-claude-sonnet-4-5"),
            "claude-sonnet-4-5"
        ); // version normalization happens later
        assert_eq!(strip_common_prefixes("headless-goose-o3-mini"), "o3-mini");
        assert_eq!(
            strip_common_prefixes("kgoose-cashapp-claude-4-sonnet"),
            "claude-4-sonnet"
        );
        assert_eq!(
            strip_common_prefixes("claude-3-5-sonnet"),
            "claude-3-5-sonnet"
        ); // no prefix

        // Test new provider patterns
        assert_eq!(
            strip_common_prefixes("databricks-deepseek-chat"),
            "deepseek-chat"
        );
        assert_eq!(strip_common_prefixes("goose-qwen-2-5-72b"), "qwen-2-5-72b");
        assert_eq!(strip_common_prefixes("kgoose-grok-4-fast"), "grok-4-fast");
        assert_eq!(
            strip_common_prefixes("databricks-jamba-large"),
            "jamba-large"
        );
        assert_eq!(
            strip_common_prefixes("goose-command-r-plus"),
            "command-r-plus"
        );
        assert_eq!(strip_common_prefixes("databricks-codestral"), "codestral");
        assert_eq!(strip_common_prefixes("goose-ministral-8b"), "ministral-8b");
    }

    #[test]
    fn test_extract_provider_prefix() {
        assert_eq!(
            extract_provider_prefix("anthropic-claude-3-5-sonnet"),
            Some(("anthropic", "claude-3-5-sonnet"))
        );
        assert_eq!(
            extract_provider_prefix("openai-gpt-4o"),
            Some(("openai", "gpt-4o"))
        );
        assert_eq!(
            extract_provider_prefix("google-gemini-2-5-flash"),
            Some(("google", "gemini-2-5-flash"))
        );
        assert_eq!(
            extract_provider_prefix("meta-llama-3-1-70b"),
            Some(("meta-llama", "3-1-70b"))
        );
        assert_eq!(
            extract_provider_prefix("mistralai-mistral-large"),
            Some(("mistralai", "mistral-large"))
        );
        assert_eq!(
            extract_provider_prefix("deepseek-deepseek-chat"),
            Some(("deepseek", "deepseek-chat"))
        );
        assert_eq!(
            extract_provider_prefix("qwen-qwen-2-5-72b-instruct"),
            Some(("qwen", "qwen-2-5-72b-instruct"))
        );
        assert_eq!(
            extract_provider_prefix("x-ai-grok-3"),
            Some(("x-ai", "grok-3"))
        );
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
        assert_eq!(
            swap_claude_word_order("claude-3.5-sonnet"),
            Some("claude-sonnet-3.5".to_string())
        );
        assert_eq!(
            swap_claude_word_order("claude-3.5-haiku"),
            Some("claude-haiku-3.5".to_string())
        );
        assert_eq!(
            swap_claude_word_order("claude-3.7-sonnet"),
            Some("claude-sonnet-3.7".to_string())
        );

        // Claude 3 series with dashes in version (before normalization)
        assert_eq!(
            swap_claude_word_order("claude-3-5-sonnet"),
            Some("claude-sonnet-3-5".to_string())
        );
        assert_eq!(
            swap_claude_word_order("claude-3-7-sonnet"),
            Some("claude-sonnet-3-7".to_string())
        );

        // Reverse: size-version to version-size (with dashes)
        assert_eq!(
            swap_claude_word_order("claude-haiku-3-5"),
            Some("claude-3-5-haiku".to_string())
        );
        assert_eq!(
            swap_claude_word_order("claude-sonnet-3-7"),
            Some("claude-3-7-sonnet".to_string())
        );

        // Claude 4 series: size-version to version-size
        assert_eq!(
            swap_claude_word_order("claude-opus-4"),
            Some("claude-4-opus".to_string())
        );
        assert_eq!(
            swap_claude_word_order("claude-sonnet-4"),
            Some("claude-4-sonnet".to_string())
        );
        assert_eq!(
            swap_claude_word_order("claude-haiku-4.5"),
            Some("claude-4.5-haiku".to_string())
        );
        assert_eq!(
            swap_claude_word_order("claude-sonnet-4.5"),
            Some("claude-4.5-sonnet".to_string())
        );

        // Claude 4 series reverse: version-size to size-version
        assert_eq!(
            swap_claude_word_order("claude-4-opus"),
            Some("claude-opus-4".to_string())
        );
        assert_eq!(
            swap_claude_word_order("claude-4-sonnet"),
            Some("claude-sonnet-4".to_string())
        );

        // Non-claude models should return None
        assert_eq!(swap_claude_word_order("gpt-4o"), None);
        assert_eq!(swap_claude_word_order("gemini-2.5-flash"), None);
    }

    #[test]
    fn test_strip_version_suffix_special_cases() {
        // Test -bedrock suffix
        assert_eq!(
            strip_version_suffix("claude-4-sonnet-bedrock"),
            "claude-4-sonnet"
        );

        // Ensure we don't strip the main version number
        assert_eq!(strip_version_suffix("claude-4"), "claude-4");
        assert_eq!(strip_version_suffix("gpt-4"), "gpt-4");
    }
}
