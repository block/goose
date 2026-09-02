use rmcp::model::Tool;
use std::any::Any;
use std::sync::{Arc, Mutex};

use crate::local_model_registry::ModelSettings;
use goose_provider_types::conversation::message::Message;
use goose_provider_types::errors::ProviderError;
use goose_provider_types::request_log::RequestLogHandle;

use super::{ResolvedModelPaths, StreamSender};

pub(super) trait BackendLoadedModel: Send {
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

pub(super) struct LocalGenerationRequest {
    pub model_name: String,
    pub system: String,
    pub messages: Vec<Message>,
    pub tools: Vec<Tool>,
    pub settings: ModelSettings,
    #[cfg_attr(not(feature = "eredu"), allow(dead_code))]
    pub temperature: Option<f32>,
    #[cfg_attr(not(feature = "eredu"), allow(dead_code))]
    pub max_tokens: Option<i32>,
    pub context_limit: usize,
    #[cfg_attr(not(feature = "eredu"), allow(dead_code))]
    pub model_load_ms: Option<u64>,
    #[cfg_attr(not(feature = "llamacpp"), allow(dead_code))]
    pub resolved_model: ResolvedModelPaths,
    pub message_id: String,
    pub tx: StreamSender,
    pub log: Arc<Mutex<Option<Box<dyn RequestLogHandle>>>>,
}

pub(super) trait LocalInferenceBackend: Send + Sync {
    fn id(&self) -> &'static str;

    fn load_model(
        &self,
        model_id: &str,
        resolved: &ResolvedModelPaths,
        settings: &ModelSettings,
    ) -> Result<Box<dyn BackendLoadedModel>, ProviderError>;

    fn generate(
        &self,
        loaded: &mut dyn BackendLoadedModel,
        request: LocalGenerationRequest,
    ) -> Result<(), ProviderError>;

    fn available_memory_bytes(&self) -> u64;
}
