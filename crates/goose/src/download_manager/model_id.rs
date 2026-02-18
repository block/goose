use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ModelIdError {
    #[error("Invalid model ID format: {0}. Expected format: org/model or org/model:variant")]
    InvalidFormat(String),
    #[error("Empty organization name")]
    EmptyOrganization,
    #[error("Empty model name")]
    EmptyModelName,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelIdentifier {
    pub original: String,
    pub organization: String,
    pub model_name: String,
    pub variant: Option<String>,
    pub normalized: String,
}

impl ModelIdentifier {
    /// Parse a HuggingFace model ID in the format: org/model or org/model:variant
    ///
    /// Examples:
    /// - "meta-llama/Llama-3.1-8B" -> org=meta-llama, model=Llama-3.1-8B, variant=None
    /// - "TheBloke/Llama-2-7B-GGUF:Q4_K_M" -> org=TheBloke, model=Llama-2-7B-GGUF, variant=Q4_K_M
    pub fn parse(model_id: &str) -> Result<Self, ModelIdError> {
        let parts: Vec<&str> = model_id.split('/').collect();
        if parts.len() != 2 {
            return Err(ModelIdError::InvalidFormat(model_id.to_string()));
        }

        let organization = parts[0].trim();
        if organization.is_empty() {
            return Err(ModelIdError::EmptyOrganization);
        }

        let model_part = parts[1].trim();
        if model_part.is_empty() {
            return Err(ModelIdError::EmptyModelName);
        }

        let (model_name, variant) = if let Some(colon_pos) = model_part.find(':') {
            let (name, var) = model_part.split_at(colon_pos);
            // Safe: ':' is ASCII, so var[1..] is valid
            #[allow(clippy::string_slice)]
            let variant_str = &var[1..]; // Skip the ':'
            if variant_str.is_empty() {
                (name, None)
            } else {
                (name, Some(variant_str.to_string()))
            }
        } else {
            (model_part, None)
        };

        if model_name.is_empty() {
            return Err(ModelIdError::EmptyModelName);
        }

        let mut identifier = Self {
            original: model_id.to_string(),
            organization: organization.to_string(),
            model_name: model_name.to_string(),
            variant,
            normalized: String::new(), // Will be set below
        };

        identifier.normalized = identifier.normalize();
        Ok(identifier)
    }

    /// Normalize the model ID to a filesystem-safe string
    ///
    /// Converts to lowercase and replaces special characters with underscores
    ///
    /// Examples:
    /// - "meta-llama/Llama-3.1-8B" -> "meta-llama_llama-3.1-8b"
    /// - "TheBloke/Llama-2-7B-GGUF:Q4_K_M" -> "thebloke_llama-2-7b-gguf_q4_k_m"
    fn normalize(&self) -> String {
        let mut normalized = format!("{}_{}", self.organization, self.model_name);

        if let Some(variant) = &self.variant {
            normalized.push('_');
            normalized.push_str(variant);
        }

        // Convert to lowercase and replace special characters
        normalized
            .to_lowercase()
            .chars()
            .map(|c| match c {
                'a'..='z' | '0'..='9' | '-' | '.' => c,
                _ => '_',
            })
            .collect()
    }

    /// Build a HuggingFace download URL for a specific file
    ///
    /// Format: https://huggingface.co/{org}/{model}/resolve/main/{file}
    ///
    /// Note: The variant is not used in the URL, as it's typically part of the filename
    pub fn to_download_url(&self, file_name: &str) -> String {
        format!(
            "https://huggingface.co/{}/{}/resolve/main/{}",
            self.organization, self.model_name, file_name
        )
    }
}

impl fmt::Display for ModelIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.original)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic() {
        let id = ModelIdentifier::parse("meta-llama/Llama-3.1-8B").unwrap();
        assert_eq!(id.organization, "meta-llama");
        assert_eq!(id.model_name, "Llama-3.1-8B");
        assert_eq!(id.variant, None);
        assert_eq!(id.original, "meta-llama/Llama-3.1-8B");
    }

    #[test]
    fn test_parse_with_variant() {
        let id = ModelIdentifier::parse("TheBloke/Llama-2-7B-GGUF:Q4_K_M").unwrap();
        assert_eq!(id.organization, "TheBloke");
        assert_eq!(id.model_name, "Llama-2-7B-GGUF");
        assert_eq!(id.variant, Some("Q4_K_M".to_string()));
        assert_eq!(id.original, "TheBloke/Llama-2-7B-GGUF:Q4_K_M");
    }

    #[test]
    fn test_parse_invalid_format() {
        assert!(ModelIdentifier::parse("invalid").is_err());
        assert!(ModelIdentifier::parse("a/b/c").is_err());
        assert!(ModelIdentifier::parse("/model").is_err());
        assert!(ModelIdentifier::parse("org/").is_err());
    }

    #[test]
    fn test_normalize_basic() {
        let id = ModelIdentifier::parse("meta-llama/Llama-3.1-8B").unwrap();
        assert_eq!(id.normalized, "meta-llama_llama-3.1-8b");
    }

    #[test]
    fn test_normalize_with_variant() {
        let id = ModelIdentifier::parse("TheBloke/Llama-2-7B-GGUF:Q4_K_M").unwrap();
        assert_eq!(id.normalized, "thebloke_llama-2-7b-gguf_q4_k_m");
    }

    #[test]
    fn test_normalize_special_chars() {
        let id = ModelIdentifier::parse("org/Model@Name#123:var$iant").unwrap();
        assert_eq!(id.normalized, "org_model_name_123_var_iant");
    }

    #[test]
    fn test_to_download_url() {
        let id = ModelIdentifier::parse("meta-llama/Llama-3.1-8B").unwrap();
        assert_eq!(
            id.to_download_url("model.gguf"),
            "https://huggingface.co/meta-llama/Llama-3.1-8B/resolve/main/model.gguf"
        );
    }

    #[test]
    fn test_to_download_url_with_variant() {
        let id = ModelIdentifier::parse("TheBloke/Llama-2-7B-GGUF:Q4_K_M").unwrap();
        // Variant is not included in URL, it's typically part of the filename
        assert_eq!(
            id.to_download_url("llama-2-7b.Q4_K_M.gguf"),
            "https://huggingface.co/TheBloke/Llama-2-7B-GGUF/resolve/main/llama-2-7b.Q4_K_M.gguf"
        );
    }
}
