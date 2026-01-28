use once_cell::sync::Lazy;
use regex::Regex;

static NORMALIZE_VERSION_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"-(\d)-(\d)(-|$)").unwrap());

static STRIP_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    vec![
        Regex::new(r"-latest$").unwrap(),
        Regex::new(r"-preview(-\d+)*$").unwrap(),
        Regex::new(r"-exp(-\d+)*$").unwrap(),
        Regex::new(r":exacto$").unwrap(),
        Regex::new(r"-\d{8}$").unwrap(),
        Regex::new(r"-\d{4}-\d{2}-\d{2}$").unwrap(),
        Regex::new(r"-v\d+(\.\d+)*$").unwrap(),
        Regex::new(r"-\d{3,}$").unwrap(),
        Regex::new(r"-bedrock$").unwrap(),
    ]
});

static CLAUDE_PATTERNS: Lazy<Vec<(Regex, Regex, &'static str)>> = Lazy::new(|| {
    ["sonnet", "opus", "haiku"]
        .iter()
        .map(|&size| {
            (
                Regex::new(&format!("claude-([0-9.-]+)-{}", size)).unwrap(),
                Regex::new(&format!("claude-{}-([0-9.-]+)", size)).unwrap(),
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

/// Try to build a canonical name and check if it exists in the registry
fn try_canonical(
    provider: &str,
    model: &str,
    registry: &super::CanonicalModelRegistry,
) -> Option<String> {
    let candidate = canonical_name(provider, model);
    registry.get(&candidate).map(|_| candidate)
}

/// Try to map a provider/model pair to a canonical model
pub fn map_to_canonical_model(
    provider: &str,
    model: &str,
    registry: &super::CanonicalModelRegistry,
) -> Option<String> {
    // Try direct mapping first
    if let Some(candidate) = try_canonical(provider, model, registry) {
        return Some(candidate);
    }

    // Try with common prefixes stripped
    let model_stripped = strip_common_prefixes(model);
    if model_stripped != model {
        if let Some(candidate) = try_canonical(provider, &model_stripped, registry) {
            return Some(candidate);
        }
    }

    // Try word-order swapping for Claude models (claude-4-opus ↔ claude-opus-4)
    if let Some(swapped) = swap_claude_word_order(&model_stripped) {
        if let Some(candidate) = try_canonical(provider, &swapped, registry) {
            return Some(candidate);
        }

        if is_hosting_provider(provider) {
            if let Some(inferred) = infer_provider_from_model(&swapped) {
                if let Some(candidate) = try_canonical(inferred, &swapped, registry) {
                    return Some(candidate);
                }
            }
        }
    }

    // For hosting providers, try to infer the real provider from model name patterns
    if is_hosting_provider(provider) {
        if let Some(inferred_provider) = infer_provider_from_model(&model_stripped) {
            if let Some(candidate) = try_canonical(inferred_provider, &model_stripped, registry) {
                return Some(candidate);
            }
        }

        if let Some(inferred) = infer_provider_from_model(model) {
            if let Some(candidate) = try_canonical(inferred, model, registry) {
                return Some(candidate);
            }
        }
    }

    // For provider-prefixed models like "databricks-meta-llama-3-1-70b"
    if let Some((extracted_provider, extracted_model)) = extract_provider_prefix(&model_stripped) {
        if let Some(candidate) = try_canonical(extracted_provider, extracted_model, registry) {
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
    matches!(
        provider,
        "databricks" | "openrouter" | "azure" | "bedrock" | "chatgpt_codex"
    )
}

/// Infer the real provider from model name patterns
fn infer_provider_from_model(model: &str) -> Option<&'static str> {
    let model_lower = model.to_lowercase();

    if model_lower.contains("claude") {
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

    if model_lower.contains("deepseek") {
        return Some("deepseek");
    }

    if model_lower.contains("qwen") {
        return Some("qwen");
    }

    if model_lower.contains("grok") {
        return Some("x-ai");
    }

    if model_lower.contains("jamba") {
        return Some("ai21");
    }

    if model_lower.contains("command") {
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
        "llama-",
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
        return model.get(pos..).unwrap_or(model).to_string();
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
    fn test_map_to_canonical_model() {
        let r = super::super::CanonicalModelRegistry::bundled().unwrap();

        // === Direct provider (non-hosting) ===
        assert_eq!(
            map_to_canonical_model("anthropic", "claude-3-5-sonnet-20241022", r),
            Some("anthropic/claude-3.5-sonnet".to_string())
        );
        assert_eq!(
            map_to_canonical_model("openai", "gpt-4o-latest", r),
            Some("openai/gpt-4o".to_string())
        );
        assert_eq!(
            map_to_canonical_model("openai", "gpt-4-turbo-2024-04-09", r),
            Some("openai/gpt-4-turbo".to_string())
        );

        // === OpenRouter (already canonical format) ===
        assert_eq!(
            map_to_canonical_model("openrouter", "anthropic/claude-3.5-sonnet", r),
            Some("anthropic/claude-3.5-sonnet".to_string())
        );

        // === Anthropic Claude - basic ===
        assert_eq!(
            map_to_canonical_model("databricks", "claude-3-5-sonnet", r),
            Some("anthropic/claude-3.5-sonnet".to_string())
        );
        assert_eq!(
            map_to_canonical_model("databricks", "claude-3-5-sonnet-20241022", r),
            Some("anthropic/claude-3.5-sonnet".to_string())
        );
        assert_eq!(
            map_to_canonical_model("databricks", "claude-3-5-sonnet-latest", r),
            Some("anthropic/claude-3.5-sonnet".to_string())
        );

        // 3.x: {model}-{version} → {version}-{model}
        assert_eq!(
            map_to_canonical_model("databricks", "claude-haiku-3-5", r),
            Some("anthropic/claude-3.5-haiku".to_string())
        );

        // 4.x: {version}-{model} → {model}-{version}
        assert_eq!(
            map_to_canonical_model("databricks", "claude-4-sonnet", r),
            Some("anthropic/claude-sonnet-4".to_string())
        );

        // 4.x with minor version + prefix stripping
        assert_eq!(
            map_to_canonical_model("databricks", "raml-claude-opus-4-5", r),
            Some("anthropic/claude-opus-4.5".to_string())
        );

        // === Claude with platform suffixes ===
        assert_eq!(
            map_to_canonical_model("databricks", "claude-4-sonnet-bedrock", r),
            Some("anthropic/claude-sonnet-4".to_string())
        );
        assert_eq!(
            map_to_canonical_model("databricks", "goose-claude-4-sonnet-bedrock", r),
            Some("anthropic/claude-sonnet-4".to_string())
        );
        assert_eq!(
            map_to_canonical_model("bedrock", "claude-3-5-sonnet", r),
            Some("anthropic/claude-3.5-sonnet".to_string())
        );

        // === OpenAI GPT ===
        assert_eq!(
            map_to_canonical_model("databricks", "gpt-4o", r),
            Some("openai/gpt-4o".to_string())
        );
        assert_eq!(
            map_to_canonical_model("databricks", "gpt-4o-2024-11-20", r),
            Some("openai/gpt-4o".to_string())
        );
        assert_eq!(
            map_to_canonical_model("databricks", "gpt-4o-latest", r),
            Some("openai/gpt-4o".to_string())
        );
        assert_eq!(
            map_to_canonical_model("databricks", "kgoose-gpt-4o", r),
            Some("openai/gpt-4o".to_string())
        );
        assert_eq!(
            map_to_canonical_model("azure", "gpt-4o", r),
            Some("openai/gpt-4o".to_string())
        );

        // === OpenAI O-series ===
        assert_eq!(
            map_to_canonical_model("databricks", "goose-o1", r),
            Some("openai/o1".to_string())
        );
        assert_eq!(
            map_to_canonical_model("databricks", "kgoose-o3", r),
            Some("openai/o3".to_string())
        );
        assert_eq!(
            map_to_canonical_model("databricks", "headless-goose-o3-mini", r),
            Some("openai/o3-mini".to_string())
        );

        // === Google Gemini ===
        assert_eq!(
            map_to_canonical_model("databricks", "gemini-2-5-flash", r),
            Some("google/gemini-2.5-flash".to_string())
        );

        // === Meta Llama ===
        assert_eq!(
            map_to_canonical_model("databricks", "meta-llama-3-1-70b-instruct", r),
            Some("meta-llama/llama-3.1-70b-instruct".to_string())
        );

        // === Mistral variants ===
        assert_eq!(
            map_to_canonical_model("databricks", "codestral", r),
            Some("mistralai/codestral".to_string())
        );
        assert_eq!(
            map_to_canonical_model("databricks", "ministral-8b", r),
            Some("mistralai/ministral-8b".to_string())
        );

        // === DeepSeek ===
        assert_eq!(
            map_to_canonical_model("databricks", "databricks-deepseek-chat", r),
            Some("deepseek/deepseek-chat".to_string())
        );
        assert_eq!(
            map_to_canonical_model("databricks", "deepseek-r1", r),
            Some("deepseek/deepseek-r1".to_string())
        );

        // === Qwen ===
        assert_eq!(
            map_to_canonical_model("databricks", "qwen-2-5-72b-instruct", r),
            Some("qwen/qwen-2.5-72b-instruct".to_string())
        );
        assert_eq!(
            map_to_canonical_model("databricks", "goose-qwen-2-5-72b-instruct", r),
            Some("qwen/qwen-2.5-72b-instruct".to_string())
        );

        // === Grok (X.AI) ===
        assert_eq!(
            map_to_canonical_model("databricks", "grok-3", r),
            Some("x-ai/grok-3".to_string())
        );
        assert_eq!(
            map_to_canonical_model("databricks", "databricks-grok-4-fast", r),
            Some("x-ai/grok-4-fast".to_string())
        );
        assert_eq!(
            map_to_canonical_model("databricks", "kgoose-grok-4-fast", r),
            Some("x-ai/grok-4-fast".to_string())
        );

        // === Jamba (AI21) ===
        assert_eq!(
            map_to_canonical_model("databricks", "jamba-large-1-7", r),
            Some("ai21/jamba-large-1.7".to_string())
        );
        assert_eq!(
            map_to_canonical_model("databricks", "databricks-jamba-large-1-7", r),
            Some("ai21/jamba-large-1.7".to_string())
        );

        // === Cohere Command ===
        assert_eq!(
            map_to_canonical_model("databricks", "command-r-plus-08", r),
            Some("cohere/command-r-plus-08".to_string())
        );
        assert_eq!(
            map_to_canonical_model("databricks", "goose-command-r-08", r),
            Some("cohere/command-r-08".to_string())
        );

        // === Provider-prefixed extraction ===
        assert_eq!(
            map_to_canonical_model("databricks", "anthropic-claude-3-5-sonnet", r),
            Some("anthropic/claude-3.5-sonnet".to_string())
        );
        assert_eq!(
            map_to_canonical_model("databricks", "openai-gpt-4o", r),
            Some("openai/gpt-4o".to_string())
        );
        assert_eq!(
            map_to_canonical_model("databricks", "google-gemini-2-5-flash", r),
            Some("google/gemini-2.5-flash".to_string())
        );
        assert_eq!(
            map_to_canonical_model("databricks", "mistralai-mistral-large", r),
            Some("mistralai/mistral-large".to_string())
        );
        assert_eq!(
            map_to_canonical_model("databricks", "deepseek-deepseek-chat", r),
            Some("deepseek/deepseek-chat".to_string())
        );
        assert_eq!(
            map_to_canonical_model("databricks", "qwen-qwen-2-5-72b-instruct", r),
            Some("qwen/qwen-2.5-72b-instruct".to_string())
        );
        assert_eq!(
            map_to_canonical_model("databricks", "x-ai-grok-3", r),
            Some("x-ai/grok-3".to_string())
        );
    }
}
