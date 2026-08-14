//! Prepares system prompts and tools before provider inference.

#[cfg(feature = "code-mode")]
use std::sync::Arc;

use crate::agents::state_machine::InferenceInput;
#[cfg(feature = "code-mode")]
use crate::agents::ExtensionManager;
use crate::agents::PromptManager;
use crate::config::GooseMode;
use crate::session::Session;
use crate::tool_inspection::ToolInspectionManager;
use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::Mutex;

pub struct PreparedInferenceRequest {
    pub(super) system_prompt: String,
    pub(super) tools: Vec<rmcp::model::Tool>,
    pub(super) moim_parts: Vec<String>,
}

#[async_trait]
pub trait InferenceRequestPreparer<S>: Send + Sync {
    async fn prepare(&self, session: &S, input: InferenceInput)
        -> Result<PreparedInferenceRequest>;
}

pub(super) struct IdentityInferenceRequestPreparer;

#[async_trait]
impl<S: Sync> InferenceRequestPreparer<S> for IdentityInferenceRequestPreparer {
    async fn prepare(
        &self,
        _session: &S,
        input: InferenceInput,
    ) -> Result<PreparedInferenceRequest> {
        Ok(PreparedInferenceRequest {
            system_prompt: input
                .prompt_parts
                .into_iter()
                .map(|(_, part)| part)
                .collect::<Vec<_>>()
                .join("\n\n"),
            tools: input.tools,
            moim_parts: input.moim_parts,
        })
    }
}

pub struct GooseInferenceRequestPreparer<'a> {
    #[cfg(feature = "code-mode")]
    pub(super) extension_manager: Arc<ExtensionManager>,
    pub(super) goose_mode: &'a Mutex<GooseMode>,
    pub(super) prompt_manager: &'a Mutex<PromptManager>,
    pub(super) tool_inspection_manager: &'a ToolInspectionManager,
    pub(super) frontend_instructions: &'a Mutex<Option<String>>,
}

#[async_trait]
impl InferenceRequestPreparer<Session> for GooseInferenceRequestPreparer<'_> {
    async fn prepare(
        &self,
        session: &Session,
        mut input: InferenceInput,
    ) -> Result<PreparedInferenceRequest> {
        #[cfg(feature = "code-mode")]
        let code_execution_mode = self
            .extension_manager
            .is_extension_enabled(
                crate::agents::platform_extensions::code_execution::EXTENSION_NAME,
            )
            .await;
        #[cfg(not(feature = "code-mode"))]
        let code_execution_mode = false;

        let goose_mode = *self.goose_mode.lock().await;
        if goose_mode == GooseMode::SmartApprove {
            self.tool_inspection_manager
                .apply_tool_annotations(&input.tools);
        }
        let tools =
            crate::agents::reply_parts::prepare_inference_tools(input.tools, code_execution_mode);
        if let Some(frontend_instructions) = self.frontend_instructions.lock().await.clone() {
            input
                .prompt_parts
                .push(("frontend".to_string(), frontend_instructions));
        }
        let system_prompt = self.prompt_manager.lock().await.build_system_prompt(
            &session.working_dir,
            input.prompt_parts,
            goose_mode,
        );
        Ok(PreparedInferenceRequest {
            system_prompt,
            tools,
            moim_parts: input.moim_parts,
        })
    }
}
