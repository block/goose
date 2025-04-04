#[cfg(test)]
mod tests {
    use goose::providers;
    
    #[test]
    fn test_huggingface_provider_metadata() {
        let providers = providers::providers();
        let huggingface = providers.iter().find(|p| p.name == "huggingface");
        assert!(huggingface.is_some(), "HuggingFace provider not found");
        
        let metadata = huggingface.unwrap();
        assert_eq!(metadata.name, "huggingface");
        assert_eq!(metadata.display_name, "HuggingFace");
        
        // Check that required config keys are present
        let token_key = metadata.config_keys.iter().find(|k| k.name == "HUGGINGFACE_TOKEN");
        assert!(token_key.is_some(), "HUGGINGFACE_TOKEN config key not found");
        assert!(token_key.unwrap().required, "HUGGINGFACE_TOKEN should be required");
        assert!(token_key.unwrap().secret, "HUGGINGFACE_TOKEN should be secret");
        
        let provider_key = metadata.config_keys.iter().find(|k| k.name == "HUGGINGFACE_PROVIDER");
        assert!(provider_key.is_some(), "HUGGINGFACE_PROVIDER config key not found");
        assert!(!provider_key.unwrap().required, "HUGGINGFACE_PROVIDER should be optional");
        assert_eq!(provider_key.unwrap().default, Some("nebius".to_string()), "HUGGINGFACE_PROVIDER should default to 'nebius'");
    }
}