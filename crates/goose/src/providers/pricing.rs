use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

/// Disk cache configuration
const CACHE_FILE_NAME: &str = "pricing_cache.json";
const CACHE_TTL_DAYS: u64 = 7; // Cache for 7 days

/// Get the cache directory path
fn get_cache_dir() -> Result<PathBuf> {
    let cache_dir = if let Ok(goose_dir) = std::env::var("GOOSE_CACHE_DIR") {
        PathBuf::from(goose_dir)
    } else {
        dirs::cache_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine cache directory"))?
            .join("goose")
    };
    Ok(cache_dir)
}

/// Cached pricing data structure for disk storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedPricingData {
    /// Nested HashMap: provider -> model -> pricing info
    pub pricing: HashMap<String, HashMap<String, PricingInfo>>,
    /// Unix timestamp when data was fetched
    pub fetched_at: u64,
}

/// Simplified pricing info for efficient storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingInfo {
    pub input_cost: f64,  // Cost per token
    pub output_cost: f64, // Cost per token
    pub context_length: Option<u32>,
}

/// Cache for OpenRouter pricing data with disk persistence
pub struct PricingCache {
    /// In-memory cache
    memory_cache: Arc<RwLock<Option<CachedPricingData>>>,
}

impl PricingCache {
    pub fn new() -> Self {
        Self {
            memory_cache: Arc::new(RwLock::new(None)),
        }
    }

    /// Load pricing from disk cache
    async fn load_from_disk(&self) -> Result<Option<CachedPricingData>> {
        let cache_path = get_cache_dir()?.join(CACHE_FILE_NAME);

        if !cache_path.exists() {
            return Ok(None);
        }

        match tokio::fs::read(&cache_path).await {
            Ok(data) => {
                match serde_json::from_slice::<CachedPricingData>(&data) {
                    Ok(cached) => {
                        // Check if cache is still valid
                        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
                        let age_days = (now - cached.fetched_at) / (24 * 60 * 60);

                        if age_days < CACHE_TTL_DAYS {
                            tracing::debug!(
                                "Loaded pricing data from disk cache (age: {} days)",
                                age_days
                            );
                            Ok(Some(cached))
                        } else {
                            tracing::debug!("Disk cache expired (age: {} days)", age_days);
                            Ok(None)
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to parse pricing cache: {}", e);
                        Ok(None)
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Failed to read pricing cache: {}", e);
                Ok(None)
            }
        }
    }

    /// Save pricing data to disk
    async fn save_to_disk(&self, data: &CachedPricingData) -> Result<()> {
        let cache_dir = get_cache_dir()?;
        tokio::fs::create_dir_all(&cache_dir).await?;

        let cache_path = cache_dir.join(CACHE_FILE_NAME);
        let json_data = serde_json::to_vec_pretty(data)?;
        tokio::fs::write(&cache_path, json_data).await?;

        tracing::debug!("Saved pricing data to disk cache");
        Ok(())
    }

    /// Get pricing for a specific model
    pub async fn get_model_pricing(&self, provider: &str, model: &str) -> Option<PricingInfo> {
        // Try memory cache first
        {
            let cache = self.memory_cache.read().await;
            if let Some(cached) = &*cache {
                return cached
                    .pricing
                    .get(&provider.to_lowercase())
                    .and_then(|models| models.get(model))
                    .cloned();
            }
        }

        // Try loading from disk
        if let Ok(Some(disk_cache)) = self.load_from_disk().await {
            // Update memory cache
            {
                let mut cache = self.memory_cache.write().await;
                *cache = Some(disk_cache.clone());
            }

            return disk_cache
                .pricing
                .get(&provider.to_lowercase())
                .and_then(|models| models.get(model))
                .cloned();
        }

        None
    }

    /// Force refresh pricing data from OpenRouter
    pub async fn refresh(&self) -> Result<()> {
        let pricing = fetch_openrouter_pricing_internal().await?;

        // Convert to our efficient structure
        let mut structured_pricing: HashMap<String, HashMap<String, PricingInfo>> = HashMap::new();

        for (model_id, model) in pricing {
            if let Some((provider, model_name)) = parse_model_id(&model_id) {
                if let (Some(input_cost), Some(output_cost)) = (
                    convert_pricing(&model.pricing.prompt),
                    convert_pricing(&model.pricing.completion),
                ) {
                    let provider_lower = provider.to_lowercase();
                    let provider_models = structured_pricing.entry(provider_lower).or_default();

                    provider_models.insert(
                        model_name,
                        PricingInfo {
                            input_cost,
                            output_cost,
                            context_length: model.context_length,
                        },
                    );
                }
            }
        }

        let cached_data = CachedPricingData {
            pricing: structured_pricing,
            fetched_at: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
        };

        // Log how many models we fetched
        let total_models: usize = cached_data
            .pricing
            .values()
            .map(|models| models.len())
            .sum();
        tracing::debug!(
            "Fetched pricing for {} providers with {} total models from OpenRouter",
            cached_data.pricing.len(),
            total_models
        );

        // Save to disk
        self.save_to_disk(&cached_data).await?;

        // Update memory cache
        {
            let mut cache = self.memory_cache.write().await;
            *cache = Some(cached_data);
        }

        Ok(())
    }

    /// Initialize cache (load from disk or fetch if needed)
    pub async fn initialize(&self) -> Result<()> {
        // Try loading from disk first
        if let Ok(Some(cached)) = self.load_from_disk().await {
            // Log how many models we have cached
            let total_models: usize = cached.pricing.values().map(|models| models.len()).sum();
            tracing::debug!(
                "Loaded {} providers with {} total models from disk cache",
                cached.pricing.len(),
                total_models
            );

            // Update memory cache
            {
                let mut cache = self.memory_cache.write().await;
                *cache = Some(cached);
            }

            return Ok(());
        }

        // If no disk cache, fetch from OpenRouter
        tracing::info!("Fetching pricing data from OpenRouter API");
        self.refresh().await
    }
}

impl Default for PricingCache {
    fn default() -> Self {
        Self::new()
    }
}

// Global cache instance
lazy_static::lazy_static! {
    static ref PRICING_CACHE: PricingCache = PricingCache::new();
}

/// Create a properly configured HTTP client for the current runtime
fn create_http_client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(30))
        .pool_idle_timeout(Duration::from_secs(90))
        .pool_max_idle_per_host(10)
        .build()
        .expect("Failed to create HTTP client")
}

/// OpenRouter model pricing information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenRouterModel {
    pub id: String,
    pub name: String,
    pub pricing: OpenRouterPricing,
    pub context_length: Option<u32>,
    pub architecture: Option<Architecture>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenRouterPricing {
    pub prompt: String,     // Cost per token for input (in USD)
    pub completion: String, // Cost per token for output (in USD)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Architecture {
    pub modality: String,
    pub tokenizer: String,
    pub instruct_type: Option<String>,
}

/// Response from OpenRouter models endpoint
#[derive(Debug, Deserialize)]
pub struct OpenRouterModelsResponse {
    pub data: Vec<OpenRouterModel>,
}

/// Internal function to fetch pricing data
async fn fetch_openrouter_pricing_internal() -> Result<HashMap<String, OpenRouterModel>> {
    let client = create_http_client();
    let response = client
        .get("https://openrouter.ai/api/v1/models")
        .send()
        .await?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Failed to fetch OpenRouter models: HTTP {}",
            response.status()
        );
    }

    let models_response: OpenRouterModelsResponse = response.json().await?;

    // Create a map for easy lookup
    let mut pricing_map = HashMap::new();
    for model in models_response.data {
        pricing_map.insert(model.id.clone(), model);
    }

    Ok(pricing_map)
}

/// Initialize pricing cache on startup
pub async fn initialize_pricing_cache() -> Result<()> {
    PRICING_CACHE.initialize().await
}

/// Get pricing for a specific model
pub async fn get_model_pricing(provider: &str, model: &str) -> Option<PricingInfo> {
    PRICING_CACHE.get_model_pricing(provider, model).await
}

/// Force refresh pricing data
pub async fn refresh_pricing() -> Result<()> {
    PRICING_CACHE.refresh().await
}

/// Get all cached pricing data
pub async fn get_all_pricing() -> HashMap<String, HashMap<String, PricingInfo>> {
    let cache = PRICING_CACHE.memory_cache.read().await;
    if let Some(cached) = &*cache {
        cached.pricing.clone()
    } else {
        // Try loading from disk
        if let Ok(Some(disk_cache)) = PRICING_CACHE.load_from_disk().await {
            // Update memory cache
            drop(cache);
            let mut write_cache = PRICING_CACHE.memory_cache.write().await;
            *write_cache = Some(disk_cache.clone());
            disk_cache.pricing
        } else {
            HashMap::new()
        }
    }
}

/// Convert OpenRouter model ID to provider/model format
/// e.g., "anthropic/claude-3.5-sonnet" -> ("anthropic", "claude-3.5-sonnet")
pub fn parse_model_id(model_id: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = model_id.splitn(2, '/').collect();
    if parts.len() == 2 {
        // Normalize provider names to match our internal naming
        let provider = match parts[0] {
            "openai" => "openai",
            "anthropic" => "anthropic",
            "google" => "google",
            "meta-llama" => "ollama", // Meta models often run via Ollama
            "mistralai" => "mistral",
            "cohere" => "cohere",
            "perplexity" => "perplexity",
            "deepseek" => "deepseek",
            "groq" => "groq",
            "nvidia" => "nvidia",
            "microsoft" => "azure",
            "replicate" => "replicate",
            "huggingface" => "huggingface",
            _ => parts[0],
        };
        Some((provider.to_string(), parts[1].to_string()))
    } else {
        None
    }
}

/// Convert OpenRouter pricing to cost per token (already in that format)
pub fn convert_pricing(price_str: &str) -> Option<f64> {
    // OpenRouter prices are already in USD per token
    price_str.parse::<f64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_model_id() {
        assert_eq!(
            parse_model_id("anthropic/claude-3.5-sonnet"),
            Some(("anthropic".to_string(), "claude-3.5-sonnet".to_string()))
        );
        assert_eq!(
            parse_model_id("openai/gpt-4"),
            Some(("openai".to_string(), "gpt-4".to_string()))
        );
        assert_eq!(parse_model_id("invalid-format"), None);

        // Test the specific model causing issues
        assert_eq!(
            parse_model_id("anthropic/claude-sonnet-4"),
            Some(("anthropic".to_string(), "claude-sonnet-4".to_string()))
        );
    }

    #[test]
    fn test_convert_pricing() {
        assert_eq!(convert_pricing("0.000003"), Some(0.000003));
        assert_eq!(convert_pricing("0.015"), Some(0.015));
        assert_eq!(convert_pricing("invalid"), None);
    }

    #[tokio::test]
    async fn test_claude_sonnet_4_pricing_lookup() {
        // Initialize the cache to load from disk
        if let Err(e) = initialize_pricing_cache().await {
            println!("Failed to initialize pricing cache: {}", e);
            return;
        }

        // Test lookup for the specific model
        let pricing = get_model_pricing("anthropic", "claude-sonnet-4").await;

        println!(
            "Pricing lookup result for anthropic/claude-sonnet-4: {:?}",
            pricing
        );

        // Should find pricing data
        if let Some(pricing_info) = pricing {
            assert!(pricing_info.input_cost > 0.0);
            assert!(pricing_info.output_cost > 0.0);
            println!(
                "Found pricing: input={}, output={}",
                pricing_info.input_cost, pricing_info.output_cost
            );
        } else {
            // Print debug info
            let all_pricing = get_all_pricing().await;
            if let Some(anthropic_models) = all_pricing.get("anthropic") {
                println!("Available anthropic models in cache:");
                for model_name in anthropic_models.keys() {
                    println!("  {}", model_name);
                }
            }
            panic!("Expected to find pricing for anthropic/claude-sonnet-4");
        }
    }
}
