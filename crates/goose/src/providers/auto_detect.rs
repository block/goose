use crate::model::ModelConfig;

/// Detect the provider based on API key format
fn detect_provider_from_key_format(api_key: &str) -> Option<&'static str> {
    let trimmed_key = api_key.trim();

    // Anthropic keys start with sk-ant-
    if trimmed_key.starts_with("sk-ant-") {
        return Some("anthropic");
    }

    // OpenAI keys start with sk- but not sk-ant-
    if trimmed_key.starts_with("sk-") && !trimmed_key.starts_with("sk-ant-") {
        return Some("openai");
    }

    // Google keys typically start with AIza
    if trimmed_key.starts_with("AIza") {
        return Some("google");
    }

    // Groq keys start with gsk_
    if trimmed_key.starts_with("gsk_") {
        return Some("groq");
    }

    // xAI keys start with xai-
    if trimmed_key.starts_with("xai-") {
        return Some("xai");
    }

    // If we can't detect the format, return None
    None
}

pub async fn detect_provider_from_api_key(api_key: &str, disable_ollama_fallback: bool) -> Option<(String, Vec<String>)> {
    // First, try to detect the provider from the key format
    if let Some(detected_provider) = detect_provider_from_key_format(api_key) {
        // Test only the detected provider
        let env_key = match detected_provider {
            "anthropic" => "ANTHROPIC_API_KEY",
            "openai" => "OPENAI_API_KEY",
            "google" => "GOOGLE_API_KEY",
            "groq" => "GROQ_API_KEY",
            "xai" => "XAI_API_KEY",
            _ => return None,
        };

        // Store original value and set the test key
        let original_value = std::env::var(env_key).ok();
        std::env::set_var(env_key, api_key);

        let result =
            match crate::providers::create(detected_provider, ModelConfig::new_or_fail("default"))
                .await
            {
                Ok(provider) => match provider.fetch_supported_models().await {
                    Ok(Some(models)) => Some((detected_provider.to_string(), models)),
                    _ => None,
                },
                Err(_) => None,
            };

        // Restore original value
        match original_value {
            Some(val) => std::env::set_var(env_key, val),
            None => std::env::remove_var(env_key),
        }

        return result;
    }

    // If we can't detect the format, try Ollama as a fallback (unless disabled)
    // (since Ollama keys don't have a standard format)
    if disable_ollama_fallback {
        return None;
    }

    let original_value = std::env::var("OLLAMA_API_KEY").ok();
    std::env::set_var("OLLAMA_API_KEY", api_key);

    let result = match crate::providers::create("ollama", ModelConfig::new_or_fail("default")).await
    {
        Ok(provider) => match provider.fetch_supported_models().await {
            Ok(Some(models)) => Some(("ollama".to_string(), models)),
            _ => None,
        },
        Err(_) => None,
    };

    // Restore original value
    match original_value {
        Some(val) => std::env::set_var("OLLAMA_API_KEY", val),
        None => std::env::remove_var("OLLAMA_API_KEY"),
    }

    result
}
