#[cfg(test)]
mod tests {
    use goose::config::declarative_providers::register_declarative_providers;
    use goose::providers::provider_registry::ProviderRegistry;

    #[test]
    fn test_groq_provider_base_url() {
        // Create a registry and register declarative providers
        let mut registry = ProviderRegistry::new();
        register_declarative_providers(&mut registry).expect("Failed to register providers");

        // Get all provider metadata
        let all_providers = registry.all_metadata_with_types();

        // Find the groqd provider
        let groq_provider = all_providers
            .iter()
            .find(|(metadata, _)| metadata.name == "groqd")
            .expect("Groq provider (groqd) not found");

        println!("Found Groq provider: {}", groq_provider.0.name);
        println!("Display name: {}", groq_provider.0.display_name);
        println!("Provider type: {:?}", groq_provider.1);

        // Find the host config key
        let host_config = groq_provider
            .0
            .config_keys
            .iter()
            .find(|config_key| config_key.name.ends_with("_HOST"))
            .expect("No host config key found");

        println!(
            "Config key: {} = {:?}",
            host_config.name, host_config.default
        );

        // Check if the base URL is correct
        if let Some(default_url) = &host_config.default {
            assert_eq!(
                default_url, "https://api.groq.com/openai/v1",
                "Groq provider should have the correct base URL"
            );
            println!("✅ SUCCESS: Base URL is correct!");
        } else {
            panic!("❌ FAILURE: No default URL found");
        }
    }

    #[test]
    fn test_all_declarative_providers_have_correct_base_urls() {
        // Create a registry and register declarative providers
        let mut registry = ProviderRegistry::new();
        register_declarative_providers(&mut registry).expect("Failed to register providers");

        // Get all provider metadata
        let all_providers = registry.all_metadata_with_types();

        // Test that declarative providers don't show OpenAI's default base URL
        for (metadata, provider_type) in all_providers.iter() {
            if let goose::providers::base::ProviderType::Declarative = provider_type {
                println!(
                    "Testing provider: {} ({})",
                    metadata.name, metadata.display_name
                );

                // Find the host config key
                if let Some(host_config) = metadata
                    .config_keys
                    .iter()
                    .find(|config_key| config_key.name.ends_with("_HOST"))
                {
                    if let Some(default_url) = &host_config.default {
                        println!("  Config key: {} = {}", host_config.name, default_url);

                        // Check that declarative providers don't have OpenAI's default URL
                        assert_ne!(
                            default_url, "https://api.openai.com",
                            "Declarative provider '{}' should not have OpenAI's default base URL",
                            metadata.name
                        );
                        println!("  ✅ Base URL is correctly overridden");
                    }
                }
            }
        }
    }
}
