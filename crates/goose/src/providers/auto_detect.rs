use crate::model::ModelConfig;

pub async fn detect_provider_from_api_key(api_key: &str) -> Option<(String, Vec<String>)> {
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
