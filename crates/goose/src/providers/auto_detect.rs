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

    // OpenRouter keys start with sk-or-
    if trimmed_key.starts_with("sk-or-") {
        return Some("openrouter");
    }

    // If we can't detect the format, return None
    None
}

/// Test a specific provider with the API key
async fn test_provider(provider_name: &str, api_key: &str) -> Option<(String, Vec<String>)> {
    let env_key = match provider_name {
        "anthropic" => "ANTHROPIC_API_KEY",
        "openai" => "OPENAI_API_KEY",
        "google" => "GOOGLE_API_KEY",
        "groq" => "GROQ_API_KEY",
        "xai" => "XAI_API_KEY",
        "openrouter" => "OPENROUTER_API_KEY",
        "ollama" => "OLLAMA_API_KEY",
        _ => return None,
    };

    let original_value = std::env::var(env_key).ok();
    std::env::set_var(env_key, api_key);

    let result = match crate::providers::create(provider_name, ModelConfig::new_or_fail("default")).await {
        Ok(provider) => match provider.fetch_supported_models().await {
            Ok(Some(models)) => Some((provider_name.to_string(), models)),
            _ => None,
        },
        Err(_) => None,
    };

    // Restore original value
    match original_value {
        Some(val) => std::env::set_var(env_key, val),
        None => std::env::remove_var(env_key),
    }

    result
}

pub async fn detect_provider_from_api_key(api_key: &str) -> Option<(String, Vec<String>)> {
    // First, try to detect the provider from the key format
    if let Some(detected_provider) = detect_provider_from_key_format(api_key) {
        // Test the detected provider first
        if let Some(result) = test_provider(detected_provider, api_key).await {
            return Some(result);
        }
    }

    // If format detection failed or the detected provider didn't work,
    // fall back to testing all providers in parallel
    let provider_tests = vec![
        ("anthropic", "ANTHROPIC_API_KEY"),
        ("openai", "OPENAI_API_KEY"),
        ("google", "GOOGLE_API_KEY"),
        ("groq", "GROQ_API_KEY"),
        ("xai", "XAI_API_KEY"),
        ("ollama", "OLLAMA_API_KEY"),
    ];

    let tasks: Vec<_> = provider_tests
        .into_iter()
        .map(|(provider_name, env_key)| {
            let api_key = api_key.to_string();
            tokio::spawn(async move {
                let original_value = std::env::var(env_key).ok();
                std::env::set_var(env_key, &api_key);

                let result = match crate::providers::create(
                    provider_name,
                    ModelConfig::new_or_fail("default"),
                )
                .await
                {
                    Ok(provider) => match provider.fetch_supported_models().await {
                        Ok(Some(models)) => Some((provider_name.to_string(), models)),
                        _ => None,
                    },
                    Err(_) => None,
                };

                match original_value {
                    Some(val) => std::env::set_var(env_key, val),
                    None => std::env::remove_var(env_key),
                }

                result
            })
        })
        .collect();

    for task in tasks {
        if let Ok(Some(result)) = task.await {
            return Some(result);
        }
    }

    None
}

/// Detect provider from API key, testing only cloud providers (no Ollama)
/// This is useful for Quick Setup flows where Ollama fallback is not desired
pub async fn detect_cloud_provider_from_api_key(api_key: &str) -> Option<(String, Vec<String>)> {
    // First, try to detect the provider from the key format
    if let Some(detected_provider) = detect_provider_from_key_format(api_key) {
        // Skip Ollama in cloud-only mode
        if detected_provider != "ollama" {
            if let Some(result) = test_provider(detected_provider, api_key).await {
                return Some(result);
            }
        }
    }

    // If format detection failed or the detected provider didn't work,
    // fall back to testing cloud providers in parallel (excluding Ollama)
    let provider_tests = vec![
        ("anthropic", "ANTHROPIC_API_KEY"),
        ("openai", "OPENAI_API_KEY"),
        ("google", "GOOGLE_API_KEY"),
        ("groq", "GROQ_API_KEY"),
        ("xai", "XAI_API_KEY"),
        // Ollama excluded for cloud-only detection
    ];

    let tasks: Vec<_> = provider_tests
        .into_iter()
        .map(|(provider_name, env_key)| {
            let api_key = api_key.to_string();
            tokio::spawn(async move {
                let original_value = std::env::var(env_key).ok();
                std::env::set_var(env_key, &api_key);

                let result = match crate::providers::create(
                    provider_name,
                    ModelConfig::new_or_fail("default"),
                )
                .await
                {
                    Ok(provider) => match provider.fetch_supported_models().await {
                        Ok(Some(models)) => Some((provider_name.to_string(), models)),
                        _ => None,
                    },
                    Err(_) => None,
                };

                match original_value {
                    Some(val) => std::env::set_var(env_key, val),
                    None => std::env::remove_var(env_key),
                }

                result
            })
        })
        .collect();

    for task in tasks {
        if let Ok(Some(result)) = task.await {
            return Some(result);
        }
    }

    None
}
