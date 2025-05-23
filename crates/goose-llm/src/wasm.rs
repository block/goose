#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;
use crate::model::ModelConfig;
use crate::message::{Message, Contents, MessageContent};
use crate::types::core::{Role, TextContent, ImageContent};
use crate::types::completion::{CompletionRequest, CompletionResponse, ExtensionConfig, ToolConfig, ToolApprovalMode, RuntimeMetrics};
use crate::providers::Usage;
use chrono::Utc;

/// WebAssembly bindings for ModelConfig
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub struct WasmModelConfig {
    inner: ModelConfig
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl WasmModelConfig {
    #[cfg_attr(feature = "wasm", wasm_bindgen(constructor))]
    pub fn new(model_name: String) -> Self {
        Self {
            inner: ModelConfig::new(model_name)
        }
    }

    #[cfg_attr(feature = "wasm", wasm_bindgen)]
    pub fn with_context_limit(self, limit: Option<u32>) -> WasmModelConfig {
        Self {
            inner: self.inner.with_context_limit(limit)
        }
    }

    #[cfg_attr(feature = "wasm", wasm_bindgen)]
    pub fn with_temperature(self, temp: Option<f32>) -> WasmModelConfig {
        Self {
            inner: self.inner.with_temperature(temp)
        }
    }

    #[cfg_attr(feature = "wasm", wasm_bindgen)]
    pub fn with_max_tokens(self, tokens: Option<i32>) -> WasmModelConfig {
        Self {
            inner: self.inner.with_max_tokens(tokens)
        }
    }

    #[cfg_attr(feature = "wasm", wasm_bindgen)]
    pub fn context_limit(&self) -> u32 {
        self.inner.context_limit()
    }

    #[cfg_attr(feature = "wasm", wasm_bindgen)]
    pub fn model_name(&self) -> String {
        self.inner.model_name.clone()
    }
}

/// WebAssembly bindings for Role enum
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub enum WasmRole {
    User,
    Assistant,
}

impl From<WasmRole> for Role {
    fn from(role: WasmRole) -> Self {
        match role {
            WasmRole::User => Role::User,
            WasmRole::Assistant => Role::Assistant,
        }
    }
}

impl From<Role> for WasmRole {
    fn from(role: Role) -> Self {
        match role {
            Role::User => WasmRole::User,
            Role::Assistant => WasmRole::Assistant,
        }
    }
}

/// WebAssembly bindings for Message
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub struct WasmMessage {
    inner: Message,
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl WasmMessage {
    #[cfg_attr(feature = "wasm", wasm_bindgen(constructor))]
    pub fn new(role: WasmRole) -> Self {
        Self {
            inner: Message::new(role.into())
        }
    }

    #[cfg_attr(feature = "wasm", wasm_bindgen)]
    pub fn user() -> Self {
        Self {
            inner: Message::user()
        }
    }

    #[cfg_attr(feature = "wasm", wasm_bindgen)]
    pub fn assistant() -> Self {
        Self {
            inner: Message::assistant()
        }
    }

    #[cfg_attr(feature = "wasm", wasm_bindgen)]
    pub fn with_text(self, text: String) -> Self {
        Self {
            inner: self.inner.with_text(text)
        }
    }

    #[cfg_attr(feature = "wasm", wasm_bindgen)]
    pub fn with_image(self, data: String, mime_type: String) -> Self {
        Self {
            inner: self.inner.with_image(data, mime_type)
        }
    }

    #[cfg_attr(feature = "wasm", wasm_bindgen)]
    pub fn role(&self) -> WasmRole {
        self.inner.role.clone().into()
    }

    #[cfg_attr(feature = "wasm", wasm_bindgen)]
    pub fn created(&self) -> i64 {
        self.inner.created
    }

    // Add a JavaScript-friendly timestamp method that returns a number 
    // instead of a BigInt for easier use with Date
    #[cfg_attr(feature = "wasm", wasm_bindgen)]
    pub fn created_ms(&self) -> f64 {
        self.inner.created as f64
    }

    #[cfg_attr(feature = "wasm", wasm_bindgen)]
    pub fn content_text(&self) -> String {
        self.inner.content.concat_text_str()
    }

    #[cfg_attr(feature = "wasm", wasm_bindgen)]
    pub fn to_json(&self) -> Result<String, JsValue> {
        serde_json::to_string(&self.inner)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    #[cfg_attr(feature = "wasm", wasm_bindgen)]
    pub fn from_json(json: &str) -> Result<WasmMessage, JsValue> {
        let message: Message = serde_json::from_str(json)
            .map_err(|e| JsValue::from_str(&format!("Deserialization error: {}", e)))?;
        
        Ok(WasmMessage { inner: message })
    }
}

/// WebAssembly bindings for ToolApprovalMode
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub enum WasmToolApprovalMode {
    Auto,
    Manual,
    Smart,
}

impl From<WasmToolApprovalMode> for ToolApprovalMode {
    fn from(mode: WasmToolApprovalMode) -> Self {
        match mode {
            WasmToolApprovalMode::Auto => ToolApprovalMode::Auto,
            WasmToolApprovalMode::Manual => ToolApprovalMode::Manual,
            WasmToolApprovalMode::Smart => ToolApprovalMode::Smart,
        }
    }
}

impl From<ToolApprovalMode> for WasmToolApprovalMode {
    fn from(mode: ToolApprovalMode) -> Self {
        match mode {
            ToolApprovalMode::Auto => WasmToolApprovalMode::Auto,
            ToolApprovalMode::Manual => WasmToolApprovalMode::Manual,
            ToolApprovalMode::Smart => WasmToolApprovalMode::Smart,
        }
    }
}

/// WebAssembly bindings for ToolConfig
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub struct WasmToolConfig {
    inner: ToolConfig,
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl WasmToolConfig {
    #[cfg_attr(feature = "wasm", wasm_bindgen(constructor))]
    pub fn new(name: String, description: String, input_schema_json: String, approval_mode: WasmToolApprovalMode) -> Result<WasmToolConfig, JsValue> {
        let input_schema = serde_json::from_str(&input_schema_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid JSON schema: {}", e)))?;
        
        Ok(Self {
            inner: ToolConfig::new(&name, &description, input_schema, approval_mode.into()),
        })
    }

    #[cfg_attr(feature = "wasm", wasm_bindgen)]
    pub fn name(&self) -> String {
        self.inner.name.clone()
    }

    #[cfg_attr(feature = "wasm", wasm_bindgen)]
    pub fn description(&self) -> String {
        self.inner.description.clone()
    }

    #[cfg_attr(feature = "wasm", wasm_bindgen)]
    pub fn input_schema_json(&self) -> Result<String, JsValue> {
        serde_json::to_string(&self.inner.input_schema)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize schema: {}", e)))
    }

    #[cfg_attr(feature = "wasm", wasm_bindgen)]
    pub fn approval_mode(&self) -> WasmToolApprovalMode {
        self.inner.approval_mode.clone().into()
    }
}

/// WebAssembly bindings for ExtensionConfig
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub struct WasmExtensionConfig {
    name: String,
    instructions: Option<String>,
    tools: Vec<ToolConfig>,
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl WasmExtensionConfig {
    #[cfg_attr(feature = "wasm", wasm_bindgen(constructor))]
    pub fn new(name: String, instructions: Option<String>) -> Self {
        Self {
            name,
            instructions,
            tools: Vec::new(),
        }
    }

    #[cfg_attr(feature = "wasm", wasm_bindgen)]
    pub fn add_tool(&mut self, tool: WasmToolConfig) {
        self.tools.push(tool.inner.clone());
    }

    #[cfg_attr(feature = "wasm", wasm_bindgen)]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    #[cfg_attr(feature = "wasm", wasm_bindgen)]
    pub fn instructions(&self) -> Option<String> {
        self.instructions.clone()
    }
    
    // Convert to the internal ExtensionConfig type
    fn to_extension_config(&self) -> ExtensionConfig {
        ExtensionConfig::new(self.name.clone(), self.instructions.clone(), self.tools.clone())
    }
}

/// WebAssembly bindings for Usage
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub struct WasmUsage {
    inner: Usage,
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl WasmUsage {
    #[cfg_attr(feature = "wasm", wasm_bindgen(constructor))]
    pub fn new(input_tokens: Option<i32>, output_tokens: Option<i32>, total_tokens: Option<i32>) -> Self {
        Self {
            inner: Usage::new(input_tokens, output_tokens, total_tokens),
        }
    }

    #[cfg_attr(feature = "wasm", wasm_bindgen)]
    pub fn input_tokens(&self) -> Option<i32> {
        self.inner.input_tokens
    }

    #[cfg_attr(feature = "wasm", wasm_bindgen)]
    pub fn output_tokens(&self) -> Option<i32> {
        self.inner.output_tokens
    }

    #[cfg_attr(feature = "wasm", wasm_bindgen)]
    pub fn total_tokens(&self) -> Option<i32> {
        self.inner.total_tokens
    }
}

/// WebAssembly bindings for RuntimeMetrics
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub struct WasmRuntimeMetrics {
    inner: RuntimeMetrics,
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl WasmRuntimeMetrics {
    #[cfg_attr(feature = "wasm", wasm_bindgen(constructor))]
    pub fn new(total_time_sec: f32, total_time_sec_provider: f32, tokens_per_second: Option<f64>) -> Self {
        Self {
            inner: RuntimeMetrics::new(total_time_sec, total_time_sec_provider, tokens_per_second),
        }
    }

    #[cfg_attr(feature = "wasm", wasm_bindgen)]
    pub fn total_time_sec(&self) -> f32 {
        self.inner.total_time_sec
    }

    #[cfg_attr(feature = "wasm", wasm_bindgen)]
    pub fn total_time_sec_provider(&self) -> f32 {
        self.inner.total_time_sec_provider
    }

    #[cfg_attr(feature = "wasm", wasm_bindgen)]
    pub fn tokens_per_second(&self) -> Option<f64> {
        self.inner.tokens_per_second
    }
}

/// WebAssembly bindings for CompletionResponse
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub struct WasmCompletionResponse {
    inner: CompletionResponse,
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl WasmCompletionResponse {
    #[cfg_attr(feature = "wasm", wasm_bindgen(constructor))]
    pub fn new(message: WasmMessage, model: String, usage: WasmUsage, runtime_metrics: WasmRuntimeMetrics) -> Self {
        Self {
            inner: CompletionResponse::new(
                message.inner.clone(),
                model,
                usage.inner.clone(),
                runtime_metrics.inner.clone(),
            ),
        }
    }

    #[cfg_attr(feature = "wasm", wasm_bindgen)]
    pub fn message(&self) -> WasmMessage {
        WasmMessage { inner: self.inner.message.clone() }
    }

    #[cfg_attr(feature = "wasm", wasm_bindgen)]
    pub fn model(&self) -> String {
        self.inner.model.clone()
    }

    #[cfg_attr(feature = "wasm", wasm_bindgen)]
    pub fn usage(&self) -> WasmUsage {
        WasmUsage { inner: self.inner.usage.clone() }
    }

    #[cfg_attr(feature = "wasm", wasm_bindgen)]
    pub fn runtime_metrics(&self) -> WasmRuntimeMetrics {
        WasmRuntimeMetrics { inner: self.inner.runtime_metrics.clone() }
    }

    #[cfg_attr(feature = "wasm", wasm_bindgen)]
    pub fn to_json(&self) -> Result<String, JsValue> {
        serde_json::to_string(&self.inner)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    #[cfg_attr(feature = "wasm", wasm_bindgen)]
    pub fn from_json(json: &str) -> Result<WasmCompletionResponse, JsValue> {
        let response: CompletionResponse = serde_json::from_str(json)
            .map_err(|e| JsValue::from_str(&format!("Deserialization error: {}", e)))?;
        
        Ok(WasmCompletionResponse { inner: response })
    }
}

/// WebAssembly bindings for CompletionRequest
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub struct WasmCompletionRequest {
    inner: CompletionRequest,
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl WasmCompletionRequest {
    #[cfg_attr(feature = "wasm", wasm_bindgen(constructor))]
    pub fn new(
        provider_name: String,
        provider_config_json: String,
        model_config: WasmModelConfig,
        system_preamble: String,
    ) -> Result<WasmCompletionRequest, JsValue> {
        let provider_config = serde_json::from_str(&provider_config_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid provider config JSON: {}", e)))?;
        
        Ok(Self {
            inner: CompletionRequest::new(
                provider_name,
                provider_config,
                model_config.inner.clone(),
                system_preamble,
                Vec::new(),
                Vec::new(),
            ),
        })
    }

    #[cfg_attr(feature = "wasm", wasm_bindgen)]
    pub fn add_message(&mut self, message: WasmMessage) {
        let mut messages = self.inner.messages.clone();
        messages.push(message.inner.clone());
        
        self.inner = CompletionRequest::new(
            self.inner.provider_name.clone(),
            self.inner.provider_config.clone(),
            self.inner.model_config.clone(),
            self.inner.system_preamble.clone(),
            messages,
            self.inner.extensions.clone(),
        );
    }

    #[cfg_attr(feature = "wasm", wasm_bindgen)]
    pub fn add_extension(&mut self, extension: WasmExtensionConfig) {
        let mut extensions = self.inner.extensions.clone();
        extensions.push(extension.to_extension_config());
        
        self.inner = CompletionRequest::new(
            self.inner.provider_name.clone(),
            self.inner.provider_config.clone(),
            self.inner.model_config.clone(),
            self.inner.system_preamble.clone(),
            self.inner.messages.clone(),
            extensions,
        );
    }

    #[cfg_attr(feature = "wasm", wasm_bindgen)]
    pub fn provider_name(&self) -> String {
        self.inner.provider_name.clone()
    }

    #[cfg_attr(feature = "wasm", wasm_bindgen)]
    pub fn provider_config_json(&self) -> Result<String, JsValue> {
        serde_json::to_string(&self.inner.provider_config)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize provider config: {}", e)))
    }

    #[cfg_attr(feature = "wasm", wasm_bindgen)]
    pub fn system_preamble(&self) -> String {
        self.inner.system_preamble.clone()
    }

    #[cfg_attr(feature = "wasm", wasm_bindgen)]
    pub fn to_json(&self) -> Result<String, JsValue> {
        serde_json::to_string(&self.inner)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    #[cfg_attr(feature = "wasm", wasm_bindgen)]
    pub fn from_json(json: &str) -> Result<WasmCompletionRequest, JsValue> {
        let request: CompletionRequest = serde_json::from_str(json)
            .map_err(|e| JsValue::from_str(&format!("Deserialization error: {}", e)))?;
        
        Ok(WasmCompletionRequest { inner: request })
    }
}