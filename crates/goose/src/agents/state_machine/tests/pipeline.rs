use std::collections::VecDeque;
use std::sync::Arc;

use anyhow::Result;
use rmcp::model::Role;
use serde_json::json;
use tokio::sync::Mutex as TokioMutex;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::calculator_extension::CalculatorExtension;
use super::dummy_api::{DummyApi, ProviderFeatures};
use crate::agents::AgentEvent;
use crate::agents::extension::ExtensionConfig;
use crate::agents::extension_manager::{ExtensionManager, ExtensionManagerCapabilities};
use crate::agents::mcp_client::McpClientTrait;
use crate::agents::prompt_manager::PromptManager;
use crate::agents::state_machine::{
    BangShellOperation, CompactionOperation, DoctorOperation, Emitter, ExitOnErrorOperation,
    InferenceRunner, MaxTurnsOperation, Operation, RecipeOperation, RetryOperation, SkillOperation,
    SlashCommandOperation, StateMachine, SteerOperation, SteerQueue, Step, StopHookOperation,
    ToolApprovalOperation, ToolExecutionOperation, ToolPairCompactionOperation,
    UnknownToolOperation,
};
use crate::config::GooseMode;
use crate::config::permission::PermissionManager;
use crate::conversation::Conversation;
use crate::conversation::message::{Message, MessageContent};
use crate::hooks::HookManager;
use crate::permission::permission_inspector::PermissionInspector;
use crate::providers::base::Provider;
use crate::session::{Session, SessionManager, SessionType};
use crate::tool_inspection::ToolInspectionManager;
use goose_providers::model::ModelConfig;

pub(super) const MAX_TURNS: u32 = 25;
pub(super) const COMPACTION_THRESHOLD: f64 = 0.8;

pub(super) struct TestPipeline {
    pub(super) session_manager: Arc<SessionManager>,
    provider: Arc<dyn Provider>,
    model_config: ModelConfig,
    extension_manager: Arc<ExtensionManager>,
    goose_mode: TokioMutex<GooseMode>,
    prompt_manager: TokioMutex<PromptManager>,
    tool_inspection_manager: ToolInspectionManager,
    frontend_instructions: TokioMutex<Option<String>>,
    hook_manager: HookManager,
    stop_hook_block_cap: u32,
    goal: TokioMutex<Option<String>>,
    grind: TokioMutex<Option<String>>,
    calculator: Arc<CalculatorExtension>,
    pub(super) session_id: String,
    steer_queue: SteerQueue,
    _temp_dir: tempfile::TempDir,
}

impl TestPipeline {
    pub(super) fn machine(&self, cancel: CancellationToken) -> StateMachine<'_> {
        let provider = self.provider.clone();
        let operations: Vec<Arc<dyn Operation + '_>> = vec![
            Arc::new(SteerOperation::new(
                self.steer_queue.clone(),
                self.hook_manager.clone(),
            )),
            Arc::new(MaxTurnsOperation::new(MAX_TURNS)),
            Arc::new(BangShellOperation::new()),
            Arc::new(CompactionOperation::new(
                provider.clone(),
                self.model_config.clone(),
                COMPACTION_THRESHOLD,
            )),
            Arc::new(ToolPairCompactionOperation::new(
                provider.clone(),
                self.model_config.clone(),
                10,
                true,
            )),
            Arc::new(ToolApprovalOperation::new(
                &self.goose_mode,
                &self.tool_inspection_manager,
            )),
            Arc::new(DoctorOperation),
            Arc::new(SkillOperation),
            Arc::new(RecipeOperation),
            Arc::new(ToolExecutionOperation::new(
                &self.goose_mode,
                self.extension_manager.clone(),
                self.hook_manager.clone(),
            )),
            Arc::new(UnknownToolOperation),
            Arc::new(RetryOperation::new(
                &self.goal,
                &self.grind,
                std::time::Duration::from_secs(1),
                std::time::Duration::from_secs(1),
            )),
            Arc::new(StopHookOperation::new(
                self.hook_manager.clone(),
                self.stop_hook_block_cap,
            )),
            Arc::new(ExitOnErrorOperation),
        ];
        let inference = Arc::new(InferenceRunner::new(
            provider,
            self.model_config.clone(),
            self.extension_manager.clone(),
            &self.goose_mode,
            &self.prompt_manager,
            &self.tool_inspection_manager,
            &self.frontend_instructions,
            self.hook_manager.clone(),
        ));
        let mut command_handlers = operations.clone();
        command_handlers.push(inference.clone());
        let command_operation: Arc<dyn Operation + '_> =
            Arc::new(SlashCommandOperation::new(command_handlers));
        let steps = std::iter::once(command_operation)
            .chain(operations)
            .map(Step::Operation)
            .chain(std::iter::once(Step::Inference(inference)))
            .collect();

        StateMachine::new(steps, cancel)
    }

    pub(super) async fn with_goose_mode(self, mode: GooseMode) -> Self {
        *self.goose_mode.lock().await = mode;
        self.session_manager
            .update(&self.session_id)
            .goose_mode(mode)
            .apply()
            .await
            .unwrap();
        self
    }

    pub(super) fn context_limit(&self) -> usize {
        self.model_config.context_limit()
    }

    pub(super) fn with_hook_manager(mut self, hook_manager: HookManager) -> Self {
        self.hook_manager = hook_manager;
        self
    }

    pub(super) fn with_stop_hook_block_cap(mut self, cap: u32) -> Self {
        self.stop_hook_block_cap = cap;
        self
    }

    pub(super) async fn set_total_tokens(&self, tokens: i32) {
        use goose_providers::conversation::token_usage::Usage;
        self.session_manager
            .update(&self.session_id)
            .usage(Usage::new(None, None, Some(tokens)))
            .apply()
            .await
            .unwrap();
    }

    pub(super) async fn set_goal(&self, goal: Option<String>) {
        *self.goal.lock().await = goal;
    }

    pub(super) async fn get_goal(&self) -> Option<String> {
        self.goal.lock().await.clone()
    }

    pub(super) async fn set_grind(&self, grind: Option<String>) {
        *self.grind.lock().await = grind;
    }

    pub(super) async fn session(&self) -> Result<Session> {
        self.session_manager
            .get_session(&self.session_id, true)
            .await
    }

    pub(super) async fn steer(&self, message: Message) {
        self.steer_queue.lock().await.push_back(message);
    }

    pub(super) async fn has_pending_steers(&self) -> bool {
        !self.steer_queue.lock().await.is_empty()
    }

    pub(super) fn calculator_total(&self) -> i64 {
        self.calculator.total()
    }

    pub(super) fn synchronize_calculator(&self, calls: usize) {
        self.calculator.synchronize(calls);
    }

    pub(super) async fn run<const N: usize>(&self, user_messages: [&str; N]) -> Result<TestRun> {
        let mut events = Vec::new();

        for text in user_messages {
            self.session_manager
                .add_message(&self.session_id, &Message::user().with_text(text))
                .await?;

            events.extend(run_machine(self).await?);
        }

        Ok(TestRun {
            session: self.session().await?,
            events,
        })
    }
}

pub(super) async fn test_pipeline() -> Result<(TestPipeline, Arc<DummyApi>)> {
    test_pipeline_with(ProviderFeatures::default()).await
}

pub(super) async fn test_pipeline_with(
    features: ProviderFeatures,
) -> Result<(TestPipeline, Arc<DummyApi>)> {
    let api = Arc::new(DummyApi::start(features).await);
    let api_client = goose_providers::api_client::ApiClient::new_with_tls(
        api.uri(),
        goose_providers::api_client::AuthMethod::NoAuth,
        None,
    )?;
    let provider: Arc<dyn Provider> = Arc::new(
        goose_providers::openai::OpenAiProviderBuilder::new(api_client)
            .name("openai")
            .build(),
    );
    let temp_dir = tempfile::tempdir()?;
    let session_manager = Arc::new(SessionManager::new(temp_dir.path().to_path_buf()));
    let shared_provider = Arc::new(TokioMutex::new(Some(provider.clone())));
    let extension_manager = Arc::new(ExtensionManager::new(
        shared_provider.clone(),
        session_manager.clone(),
        None,
        "pipeline-test".to_string(),
        ExtensionManagerCapabilities {
            mcpui: false,
            host_info: None,
        },
        false,
    ));
    let mut tool_inspection_manager = ToolInspectionManager::new();
    tool_inspection_manager.add_inspector(Box::new(PermissionInspector::new(
        Arc::new(PermissionManager::new(temp_dir.path().join("permissions"))),
        shared_provider,
        session_manager.clone(),
    )));
    let session = session_manager
        .create_session(
            temp_dir.path().to_path_buf(),
            "pipeline-test".to_string(),
            SessionType::Hidden,
            GooseMode::Auto,
        )
        .await?;
    let model_config = ModelConfig::new(goose_providers::openai::OPEN_AI_DEFAULT_MODEL)
        .with_canonical_limits("openai");
    session_manager
        .update(&session.id)
        .provider_name("openai")
        .model_config(model_config.clone())
        .apply()
        .await?;
    let calculator = Arc::new(CalculatorExtension::new(session_manager.action_required()));
    let pipeline = TestPipeline {
        session_manager,
        provider: provider.clone(),
        model_config,
        extension_manager,
        goose_mode: TokioMutex::new(GooseMode::Auto),
        prompt_manager: TokioMutex::new(PromptManager::new()),
        tool_inspection_manager,
        frontend_instructions: TokioMutex::new(None),
        hook_manager: HookManager::default(),
        stop_hook_block_cap: 3,
        goal: TokioMutex::new(None),
        grind: TokioMutex::new(None),
        calculator: calculator.clone(),
        session_id: session.id,
        steer_queue: Arc::new(tokio::sync::Mutex::new(VecDeque::new())),
        _temp_dir: temp_dir,
    };
    let extension_manager = pipeline.extension_manager.clone();
    let session_id = pipeline.session_id.clone();

    extension_manager
        .add_client(
            "calculator".to_string(),
            ExtensionConfig::Platform {
                name: "calculator".to_string(),
                description: "Stateful test calculator".to_string(),
                display_name: None,
                bundled: None,
                available_tools: vec![],
            },
            calculator.clone(),
            calculator.get_info().cloned(),
            None,
        )
        .await;
    extension_manager
        .add_extension(
            ExtensionConfig::Platform {
                name: "extensionmanager".to_string(),
                description: "Extension Manager".to_string(),
                display_name: None,
                bundled: None,
                available_tools: vec![],
            },
            Some(pipeline._temp_dir.path().to_path_buf()),
            None,
            Some(&session_id),
        )
        .await?;
    extension_manager
        .add_extension(
            ExtensionConfig::Platform {
                name: "todo".to_string(),
                description: "Todo".to_string(),
                display_name: None,
                bundled: None,
                available_tools: vec![],
            },
            Some(pipeline._temp_dir.path().to_path_buf()),
            None,
            Some(&session_id),
        )
        .await?;

    Ok((pipeline, api))
}

pub(super) struct TestRun {
    pub session: Session,
    pub events: Vec<AgentEvent>,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum MessageKind {
    Agent,
    ToolCall,
    ToolResponse,
}

impl MessageKind {
    fn prefix(self) -> &'static str {
        match self {
            Self::Agent => "agent:",
            Self::ToolCall => "toolcall:",
            Self::ToolResponse => "toolresponse:",
        }
    }
}

impl TestRun {
    pub(super) fn rendered_conversation(&self) -> Vec<String> {
        self.session
            .conversation
            .as_ref()
            .into_iter()
            .flat_map(Conversation::messages)
            .flat_map(|message| {
                message.content.iter().map(move |content| match content {
                    MessageContent::Text(text) => {
                        let role = match message.role {
                            Role::User => "user",
                            Role::Assistant => "agent",
                        };
                        format!("{role}: {}", text.text)
                    }
                    MessageContent::ToolRequest(request) => match &request.tool_call {
                        Ok(call) => {
                            let arguments = call
                                .arguments
                                .clone()
                                .map(serde_json::Value::Object)
                                .unwrap_or_else(|| json!({}));
                            format!("toolcall: {}({arguments})", call.name)
                        }
                        Err(error) => format!("toolcall error: {}", error.message),
                    },
                    MessageContent::ToolResponse(response) => {
                        let text = match response.tool_result.as_ref() {
                            Ok(result) => result
                                .content
                                .iter()
                                .filter_map(|content| {
                                    content.as_text().map(|text| text.text.clone())
                                })
                                .collect::<String>(),
                            Err(error) => error.message.to_string(),
                        };
                        format!("toolresponse: {text}")
                    }
                    MessageContent::Error(error) => format!("error: {}", error.message),
                    _ => content.to_string(),
                })
            })
            .collect()
    }

    pub(super) fn assert_message(&self, index: isize, kind: MessageKind, contains: &str) {
        let messages = self.rendered_conversation();
        let resolved = if index < 0 {
            messages.len() as isize + index
        } else {
            index
        };
        assert!(
            resolved >= 0 && resolved < messages.len() as isize,
            "message index {index} is out of bounds:\n{}",
            messages.join("\n")
        );
        let message = &messages[resolved as usize];
        let content = message.strip_prefix(kind.prefix()).unwrap_or_else(|| {
            panic!(
                "message {index} is not {kind:?}: {message}\n\n{}",
                messages.join("\n")
            )
        });
        assert!(
            content.contains(contains),
            "message {index} does not contain {contains:?}: {message}\n\n{}",
            messages.join("\n")
        );
    }
}

pub(super) async fn run_machine(pipeline: &TestPipeline) -> Result<Vec<AgentEvent>> {
    let cancel = CancellationToken::new();
    let machine = pipeline.machine(cancel.clone());
    let (tx, mut rx) = mpsc::channel(1024);
    let emit = Emitter::new(tx, cancel);
    let mut events = Vec::new();
    loop {
        let session = pipeline.session().await?;
        let Some(mut result) = machine.step(&session, emit.clone()).await? else {
            break;
        };
        machine
            .apply(
                pipeline.session_manager.as_ref(),
                &session,
                &mut result,
                &emit,
            )
            .await?;
        while let Ok(event) = rx.try_recv() {
            events.push(event);
        }
        if result.yield_to_client {
            break;
        }
    }
    drop(emit);
    while let Some(event) = rx.recv().await {
        events.push(event);
    }
    Ok(events)
}
