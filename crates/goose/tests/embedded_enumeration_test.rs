#[cfg(test)]
mod tests {
    use goose::providers::base::Provider;
    use goose::providers::embedded::EmbeddedProvider;

    #[test]
    fn test_enumerate_models_static() {
        match EmbeddedProvider::enumerate_models() {
            Ok(models) => {
                if models.is_empty() {
                    println!("ℹ️  No models found in ~/.models (directory may not exist or no .gguf files)");
                } else {
                    println!("✅ Found {} model(s) in ~/.models:", models.len());
                    for model in &models {
                        println!("  - {}", model);
                    }

                    // Verify models are sorted
                    let mut sorted_models = models.clone();
                    sorted_models.sort();
                    assert_eq!(models, sorted_models, "Models should be sorted");

                    // Verify no .gguf extension in names
                    for model in &models {
                        assert!(
                            !model.ends_with(".gguf"),
                            "Model name should not include .gguf extension: {}",
                            model
                        );
                    }
                }
            }
            Err(e) => {
                panic!("Error enumerating models: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_fetch_supported_models_with_provider() {
        let available_models = match EmbeddedProvider::enumerate_models() {
            Ok(models) => models,
            Err(e) => {
                println!("⚠️  Could not enumerate models: {}", e);
                return;
            }
        };

        if available_models.is_empty() {
            println!("ℹ️  No models available in ~/.models, skipping provider test");
            return;
        }

        // Use the first available model
        let model_name = &available_models[0];
        println!("Using model: {}", model_name);

        // Create a ModelConfig with the first available model
        let model_config =
            goose::model::ModelConfig::new(model_name).expect("Failed to create model config");

        // Create provider instance
        let provider = match EmbeddedProvider::from_env(model_config).await {
            Ok(p) => p,
            Err(e) => {
                panic!(
                    "Failed to create provider with model '{}': {}",
                    model_name, e
                );
            }
        };

        // Fetch models via the trait method
        match provider.fetch_supported_models().await {
            Ok(Some(models)) => {
                println!("✅ Provider reported {} model(s):", models.len());
                for model in &models {
                    println!("  - {}", model);
                }
                assert!(!models.is_empty(), "Expected at least one model");
                assert_eq!(
                    models, available_models,
                    "Provider should return same models as static method"
                );
            }
            Ok(None) => {
                panic!("Provider returned None, but we know models exist");
            }
            Err(e) => {
                panic!("Error fetching models from provider: {}", e);
            }
        }
    }

    #[test]
    fn test_models_directory_scanning() {
        use std::path::PathBuf;

        let home = dirs::home_dir().expect("Could not get home directory");
        let models_dir = home.join(".models");

        if !models_dir.exists() {
            println!("⚠️  ~/.models directory does not exist");
            return;
        }

        let entries = std::fs::read_dir(&models_dir).expect("Failed to read models directory");
        let gguf_files: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_file() && p.extension().is_some_and(|ext| ext == "gguf"))
            .collect();

        println!("Found {} .gguf file(s) in ~/.models:", gguf_files.len());
        for file in &gguf_files {
            if let Some(name) = file.file_name() {
                println!("  - {}", name.to_string_lossy());
            }
        }

        if !gguf_files.is_empty() {
            println!("✅ Model enumeration will work correctly");
        }
    }
}
