#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;
use crate::model::ModelConfig;
use crate::message::{Message, Contents, MessageContent};
use crate::types::core::{Role, TextContent, ImageContent};
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