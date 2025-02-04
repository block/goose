use anyhow::{Error, Result};
use async_trait::async_trait;
use ethers::prelude::*;
use reqwest::get;
use serde_json::{json, Value};
use std::ffi::c_uint;
use std::sync::Arc;
use std::time::Duration;

use super::base::{ConfigKey, Provider, ProviderMetadata, ProviderUsage, Usage};
use super::errors::ProviderError;
use super::formats::openai::{format_messages, format_tools, get_usage, response_to_message};
use super::utils::{emit_debug_trace, get_model, ImageFormat};
use crate::message::Message;
use crate::model::ModelConfig;
use mcp_core::tool::Tool;
use reqwest::{Client, Response, StatusCode};

pub const ETERNAL_AI_DEFAULT_MODEL: &str = "DeepSeek-R1-Distill-Llama-70B";
pub const ETERNAL_AI_KNOWN_MODELS: &[&str] = &[
    "DeepSeek-R1-Distill-Llama-70B",
    "neuralmagic/Meta-Llama-3.1-405B-Instruct-quantized.w4a16",
];
pub const ETERNAL_AI_DOC_URL: &str = "https://platform.openai.com/docs/models";
pub const MAPPING_CHAINID: [(&str, &str); 2] = [
    ("DeepSeek-R1-Distill-Llama-70B", "8453"),
    (
        "neuralmagic/Meta-Llama-3.1-405B-Instruct-quantized.w4a16",
        "45762",
    ),
];
const IPFS: &str = "ipfs://";
const LIGHTHOUSE_IPFS: &str = "https://gateway.lighthouse.storage/ipfs/";
const GCS_ETERNAL_AI_BASE_URL: &str = "https://cdn.eternalai.org/upload/";

#[derive(Debug, serde::Serialize)]
pub struct EternalAiProvider {
    #[serde(skip)]
    client: Client,
    host: String,
    api_key: String,
    model: ModelConfig,
    chain_id: Option<String>,
}

impl Default for EternalAiProvider {
    fn default() -> Self {
        let model = ModelConfig::new(EternalAiProvider::metadata().default_model);
        EternalAiProvider::from_env(model, None).expect("Failed to initialize EternalAi provider")
    }
}

impl EternalAiProvider {
    pub fn from_env(model: ModelConfig, chain_id: Option<String>) -> Result<Self> {
        let config = crate::config::Config::global();
        let api_key: String = config.get_secret("ETERNALAI_API_KEY")?;
        let host: String = config
            .get("ETERNALAI_HOST")
            .unwrap_or_else(|_| "https://api.eternalai.org".to_string());
        let client = Client::builder()
            .timeout(Duration::from_secs(600))
            .build()?;
        Ok(Self {
            client,
            host,
            api_key,
            model,
            chain_id,
        })
    }

    async fn post(&self, payload: Value) -> Result<Value, ProviderError> {
        let base_url = url::Url::parse(&self.host)
            .map_err(|e| ProviderError::RequestFailed(format!("Invalid base URL: {e}")))?;
        let url = base_url.join("v1/chat/completions").map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to construct endpoint URL: {e}"))
        })?;

        tracing::info!(
            "Sending request to {} \n with payload {}\n",
            url,
            payload.to_string()
        );

        let response = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&payload)
            .send()
            .await?;

        handle_response_eternalai_compat(response).await
    }
}

pub fn get_chain_id(key: String) -> Option<String> {
    for &(k, v) in &MAPPING_CHAINID {
        if k == key {
            return Some(v.to_string());
        }
    }
    Some("45762".to_string())
}

pub async fn create_eternalai_request(
    model_config: &ModelConfig,
    system: &str,
    messages: &[Message],
    tools: &[Tool],
    image_format: &ImageFormat,
    chain_id: String,
) -> Result<Value, Error> {
    let mut system_message = json!({
        "role": "system" ,
        "content": system
    });

    let eternal_ai_rpc = std::env::var("ETERNALAI_RPC_URL").unwrap_or_else(|_| "".to_string());
    let eternal_ai_contract =
        std::env::var("ETERNALAI_AGENT_CONTRACT_ADDRESS").unwrap_or_else(|_| "".to_string());
    let eternal_ai_agent_id =
        std::env::var("ETERNALAI_AGENT_ID").unwrap_or_else(|_| "".to_string());
    if !eternal_ai_rpc.is_empty()
        && !eternal_ai_contract.is_empty()
        && !eternal_ai_agent_id.is_empty()
    {
        let c_value: c_uint = eternal_ai_agent_id.parse::<u32>().unwrap_or(0);
        let prompt = match get_on_chain_system_prompt(
            &eternal_ai_rpc,
            &eternal_ai_contract,
            c_value,
        )
        .await
        {
            Ok(value) => value,
            Err(e) => return Err(Error::from(ProviderError::ExecutionError(e))),
        };
        match prompt {
            None => {
                tracing::info!("on-chain sytem prompt is none")
            }
            Some(value) => {
                tracing::info!("on-chain system prompt: {}", value);
                system_message = json!({
                    "role": "system" ,
                    "content": value,
                });
            }
        }
    }

    let messages_spec = format_messages(messages, image_format);
    let tools_spec = if !tools.is_empty() {
        format_tools(tools)?
    } else {
        vec![]
    };

    let mut messages_array = vec![system_message];
    messages_array.extend(messages_spec);

    let mut payload = json!({
        "model": model_config.model_name,
        "messages": messages_array,
        "chain_id": chain_id,
    });

    if !tools_spec.is_empty() {
        payload
            .as_object_mut()
            .unwrap()
            .insert("tools".to_string(), json!(tools_spec));
    }

    if let Some(tokens) = model_config.max_tokens {
        let key = "max_completion_tokens";
        payload
            .as_object_mut()
            .unwrap()
            .insert(key.to_string(), json!(tokens));
    }
    Ok(payload)
}

pub async fn handle_response_eternalai_compat(response: Response) -> Result<Value, ProviderError> {
    let status = response.status();
    // Try to parse the response body as JSON (if applicable)
    let payload: Option<Value> = response.json().await.ok();

    match status {
        StatusCode::OK => payload.ok_or_else( || ProviderError::RequestFailed("Response body is not valid JSON".to_string()) ),
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
            Err(ProviderError::Authentication(format!("Authentication failed. Please ensure your API keys are valid and have the required permissions. \
                Status: {}. Response: {:?}", status, payload)))
        }
        StatusCode::BAD_REQUEST => {
            let mut message = "Unknown error".to_string();
            if let Some(payload) = &payload {
                if let Some(error) = payload.get("error") {
                    tracing::debug!("Bad Request Error: {error:?}");
                    message = error
                        .get("message")
                        .and_then(|m| m.as_str())
                        .unwrap_or("Unknown error")
                        .to_string();

                    if let Some(code) = error.get("code").and_then(|c| c.as_str()) {
                        if code == "context_length_exceeded" || code == "string_above_max_length" {
                            return Err(ProviderError::ContextLengthExceeded(message));
                        }
                    }
                }}
            tracing::debug!(
                "{}", format!("Provider request failed with status: {}. Payload: {:?}", status, payload)
            );
            Err(ProviderError::RequestFailed(format!("Request failed with status: {}. Message: {}", status, message)))
        }
        StatusCode::TOO_MANY_REQUESTS => {
            Err(ProviderError::RateLimitExceeded(format!("{:?}", payload)))
        }
        StatusCode::INTERNAL_SERVER_ERROR | StatusCode::SERVICE_UNAVAILABLE => {
            Err(ProviderError::ServerError(format!("{:?}", payload)))
        }
        _ => {
            tracing::debug!(
                "{}", format!("Provider request failed with status: {}. Payload: {:?}", status, payload)
            );
            Err(ProviderError::RequestFailed(format!("Request failed with status: {}", status)))
        }
    }
}

pub async fn fetch_system_prompt_raw_or_ipfs(content: &str) -> Option<String> {
    if content.contains(IPFS) {
        let light_house = content.replace(IPFS, LIGHTHOUSE_IPFS);
        tracing::debug!("light_house : {}", light_house);
        let mut response = get(light_house).await.unwrap();
        if response.status().is_success() {
            let body = response.text().await.unwrap();
            tracing::debug!("light_house body: {}", body);
            return Some(body);
        } else {
            let gcs = content.replace(IPFS, GCS_ETERNAL_AI_BASE_URL);
            tracing::debug!("gcs: {}", gcs);
            response = get(gcs).await.unwrap();
            if response.status().is_success() {
                let body = response.text().await.unwrap();
                tracing::debug!("gcs body: {}", body);
                return Some(body);
            } else {
                return None;
            }
        }
    }
    Some(content.to_string())
}

pub async fn get_on_chain_system_prompt(
    rpc_url: &str,
    contract_addr: &str,
    agent_id: c_uint,
) -> Result<Option<String>, String> {
    abigen!(
        SystemPromptManagementContract,
        r#"
        [{"inputs": [{"internalType": "uint256", "name": "_agentId", "type": "uint256"}], "name": "getAgentSystemPrompt", "outputs": [{"internalType": "bytes[]", "name": "","type": "bytes[]"}], "stateMutability": "view", "type": "function"}]
        "#
    );
    let provider = ethers::providers::Provider::<Http>::try_from(rpc_url)
        .map_err(|e| format!("Failed to parse url: {}", e))?;
    let client = Arc::new(provider);
    let contract_address: Address = contract_addr
        .parse()
        .map_err(|e| format!("invalid contract address: {}", e))?;
    let contract = SystemPromptManagementContract::new(contract_address, client);
    let system_prompts: Vec<Bytes> = contract
        .get_agent_system_prompt(U256::from(agent_id))
        .call()
        .await
        .map_err(|e| format!("invalid agent system prompt: {}", e))?;

    let decoded_strings: Vec<String> = system_prompts
        .iter()
        .map(|bytes| {
            String::from_utf8(bytes.to_vec()).unwrap_or_else(|_| "[Invalid UTF-8]".to_string())
        })
        .collect();

    if !decoded_strings.is_empty() {
        let prompt = decoded_strings[0].clone();
        tracing::debug!("system prompt : {}", prompt);
        return Ok(fetch_system_prompt_raw_or_ipfs(&prompt).await);
    }
    Ok(None)
}

#[async_trait]
impl Provider for EternalAiProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "eternalai",
            "EternalAI",
            "DeepSeek-R1-Distill-Llama-70B and other EternalAI models",
            ETERNAL_AI_DEFAULT_MODEL,
            ETERNAL_AI_KNOWN_MODELS
                .iter()
                .map(|&s| s.to_string())
                .collect(),
            ETERNAL_AI_DOC_URL,
            vec![
                ConfigKey::new("ETERNALAI_API_KEY", true, true, None),
                ConfigKey::new(
                    "ETERNALAI_HOST",
                    false,
                    false,
                    Some("https://api.eternalai.org"),
                ),
            ],
        )
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model.clone()
    }

    #[tracing::instrument(
        skip(self, system, messages, tools),
        fields(model_config, input, output, input_tokens, output_tokens, total_tokens)
    )]
    async fn complete(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        let mut chain_id = self.chain_id.clone();
        if chain_id.is_none() {
            let chain_id_option = get_chain_id(self.model.model_name.to_string());
            match chain_id_option {
                Some(value) => {
                    chain_id = Option::from(value);
                }
                None => {
                    eprintln!("No chain ID found for model: {}", self.model.model_name);
                    return Err(ProviderError::ExecutionError(
                        "No chain ID found for model".to_string(),
                    ));
                }
            }
        }
        let payload = create_eternalai_request(
            &self.model,
            system,
            messages,
            tools,
            &ImageFormat::OpenAi,
            chain_id.unwrap().to_string(),
        )
        .await?;

        // Make request
        let response = self.post(payload.clone()).await?;

        tracing::info!("response: {:?}", response);

        // Parse response
        let message = response_to_message(response.clone())?;
        let usage = match get_usage(&response) {
            Ok(usage) => usage,
            Err(ProviderError::UsageError(e)) => {
                tracing::warn!("Failed to get usage data: {}", e);
                Usage::default()
            }
            Err(e) => return Err(e),
        };
        let model = get_model(&response);
        emit_debug_trace(self, &payload, &response, &usage);
        Ok((message, ProviderUsage::new(model, usage)))
    }
}
