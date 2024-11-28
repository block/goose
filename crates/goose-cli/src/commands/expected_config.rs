use crate::profile::{PROVIDER_DATABRICKS, PROVIDER_OLLAMA, PROVIDER_OPEN_AI};
use goose::providers::ollama::OLLAMA_MODEL;

pub struct RecommendedModels {
    pub model: &'static str,
}

pub fn get_recommended_models(provider_name: &str) -> RecommendedModels {
    if provider_name == PROVIDER_OPEN_AI {
        RecommendedModels { model: "gpt-4o" }
    } else if provider_name == PROVIDER_DATABRICKS {
        RecommendedModels {
            model: "claude-3-5-sonnet-2",
        }
    } else if provider_name == PROVIDER_OLLAMA {
        RecommendedModels {
            model: OLLAMA_MODEL,
        }
    } else {
        panic!("Invalid provider name");
    }
}
