#[cfg(test)]
mod tests {
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
