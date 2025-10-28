use anyhow::Result;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use thiserror::Error;

use crate::recipe::Recipe;

#[derive(Error, Debug)]
pub enum DecodeError {
    #[error("Failed to decode recipe deeplink")]
    AllMethodsFailed,
}

pub fn encode(recipe: &Recipe) -> Result<String, serde_json::Error> {
    let recipe_json = serde_json::to_string(recipe)?;
    let encoded = URL_SAFE_NO_PAD.encode(recipe_json.as_bytes());
    Ok(encoded)
}

pub fn decode(link: &str) -> Result<Recipe, DecodeError> {
    // Handle the current format: URL-safe Base64 without padding.
    if let Ok(decoded_bytes) = URL_SAFE_NO_PAD.decode(link) {
        if let Ok(recipe_json) = String::from_utf8(decoded_bytes) {
            if let Ok(recipe) = serde_json::from_str::<Recipe>(&recipe_json) {
                return Ok(recipe);
            }
        }
    }

    // Handle legacy formats of 'standard base64 encoded' and standard base64 encoded that was then url encoded.
    if let Ok(url_decoded) = urlencoding::decode(link) {
        if let Ok(decoded_bytes) =
            base64::engine::general_purpose::STANDARD.decode(url_decoded.as_bytes())
        {
            if let Ok(recipe_json) = String::from_utf8(decoded_bytes) {
                if let Ok(recipe) = serde_json::from_str::<Recipe>(&recipe_json) {
                    return Ok(recipe);
                }
            }
        }
    }

    Err(DecodeError::AllMethodsFailed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recipe::Recipe;

    fn create_test_recipe() -> Recipe {
        Recipe::builder()
            .title("Test Recipe")
            .description("A test recipe for deeplink encoding/decoding")
            .instructions("Act as a helpful assistant")
            .build()
            .expect("Failed to build test recipe")
    }

    #[test]
    fn test_encode_decode_round_trip() {
        let original_recipe = create_test_recipe();

        let encoded = encode(&original_recipe).expect("Failed to encode recipe");
        assert!(!encoded.is_empty());

        let decoded_recipe = decode(&encoded).expect("Failed to decode recipe");

        assert_eq!(original_recipe.title, decoded_recipe.title);
        assert_eq!(original_recipe.description, decoded_recipe.description);
        assert_eq!(original_recipe.instructions, decoded_recipe.instructions);
        assert_eq!(original_recipe.version, decoded_recipe.version);
    }

    #[test]
    fn test_decode_legacy_standard_base64() {
        let recipe = create_test_recipe();
        let recipe_json = serde_json::to_string(&recipe).unwrap();
        let legacy_encoded =
            base64::engine::general_purpose::STANDARD.encode(recipe_json.as_bytes());

        let decoded_recipe = decode(&legacy_encoded).expect("Failed to decode legacy format");
        assert_eq!(recipe.title, decoded_recipe.title);
        assert_eq!(recipe.description, decoded_recipe.description);
        assert_eq!(recipe.instructions, decoded_recipe.instructions);
    }

    #[test]
    fn test_decode_legacy_url_encoded_base64() {
        let recipe = create_test_recipe();
        let recipe_json = serde_json::to_string(&recipe).unwrap();
        let base64_encoded =
            base64::engine::general_purpose::STANDARD.encode(recipe_json.as_bytes());
        let url_encoded = urlencoding::encode(&base64_encoded);

        let decoded_recipe =
            decode(&url_encoded).expect("Failed to decode URL-encoded legacy format");
        assert_eq!(recipe.title, decoded_recipe.title);
        assert_eq!(recipe.description, decoded_recipe.description);
        assert_eq!(recipe.instructions, decoded_recipe.instructions);
    }

    #[test]
    fn test_decode_invalid_input() {
        let result = decode("invalid_base64!");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DecodeError::AllMethodsFailed));
    }
}
