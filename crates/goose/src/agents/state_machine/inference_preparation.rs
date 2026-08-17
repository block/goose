//! Goose-specific inference request preparation.

#[cfg(feature = "code-mode")]
use crate::agents::ExtensionManager;
use crate::agents::PromptManager;
use crate::config::GooseMode;
use crate::session::Session;
use crate::tool_inspection::ToolInspectionManager;
use anyhow::Result;
use async_trait::async_trait;
use goose_agent::inference::{InferenceRequestPreparer, PreparedInferenceRequest};
use goose_agent::operation::InferenceInput;
#[cfg(feature = "code-mode")]
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct GooseInferenceRequestPreparer<'a> {
    #[cfg(feature = "code-mode")]
    pub(crate) extension_manager: Arc<ExtensionManager>,
    pub(crate) goose_mode: &'a Mutex<GooseMode>,
    pub(crate) prompt_manager: &'a Mutex<PromptManager>,
    pub(crate) tool_inspection_manager: &'a ToolInspectionManager,
    pub(crate) frontend_instructions: &'a Mutex<Option<String>>,
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
