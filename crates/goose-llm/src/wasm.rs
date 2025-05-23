#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;
use crate::model::ModelConfig;

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