use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;

use goose_providers::model::ModelConfig;
use tokio_util::sync::CancellationToken;

use crate::agents::extension_manager::{ExtensionLease, ExtensionManager};
use crate::agents::state_machine::{
    subagents_finished, tool_calls_answered, BangShellOperation, CompactionOperation,
    DoctorOperation, EmptyResponseOperation, ExitOnErrorOperation, GooseEffect,
    GooseInferenceProvider, GooseInferenceRequestPreparer, InferenceRunner, MaxTurnsOperation,
    Operation, ProjectOperation, RecipeOperation, RemainingToolCallsOperation, RetryOperation,
    SkillOperation, SlashCommandOperation, StateMachine, StatusOperation, SteerOperation,
    SteerQueue, Step, StopHookOperation, SubagentOperation, ToolApprovalOperation,
    ToolExecutionOperation, ToolPairCompactionOperation,
};
use crate::agents::subagent_handler::ForegroundSubagentRunner;
use crate::hooks::HookManager;
use crate::providers::base::Provider;
use crate::session::{Session, SessionManager};
use crate::tool_inspection::ToolInspectionManager;

pub(crate) struct StateMachineConfig<'a> {
    pub(crate) provider: Arc<dyn Provider>,
    pub(crate) model_config: ModelConfig,
    pub(crate) context_limit: usize,
    pub(crate) compaction_threshold: f64,
    pub(crate) tool_call_cutoff: usize,
    pub(crate) tool_pair_summarization: bool,
    pub(crate) session_manager: Arc<SessionManager>,
    pub(crate) extension_manager: Arc<ExtensionManager>,
    pub(crate) extension_lease: Arc<StdMutex<Option<Arc<ExtensionLease>>>>,
    pub(crate) tool_inspection_manager: &'a ToolInspectionManager,
    pub(crate) hook_manager: HookManager,
    pub(crate) steer_queue: SteerQueue,
    pub(crate) subagent_runner: ForegroundSubagentRunner,
    pub(crate) retry_timeout: Duration,
    pub(crate) on_failure_timeout: Duration,
    pub(crate) stop_hook_block_cap: u32,
    pub(crate) max_turns: u32,
    pub(crate) cancel: CancellationToken,
}

pub(crate) fn build_state_machine<'a>(
    config: StateMachineConfig<'a>,
) -> StateMachine<'a, Session, GooseEffect> {
    let StateMachineConfig {
        provider,
        model_config,
        context_limit,
        compaction_threshold,
        tool_call_cutoff,
        tool_pair_summarization,
        session_manager,
        extension_manager,
        extension_lease,
        tool_inspection_manager,
        hook_manager,
        steer_queue,
        subagent_runner,
        retry_timeout,
        on_failure_timeout,
        stop_hook_block_cap,
        max_turns,
        cancel,
    } = config;
    let manages_own_context = provider.manages_own_context();
    let subagent_runner = Arc::new(subagent_runner);
    let inference_ready = || {
        vec![
            tool_calls_answered(),
            subagents_finished(subagent_runner.clone()),
        ]
    };

    let mut command_handlers = Vec::new();
    let mut step = |operation: Arc<dyn Operation<Session, GooseEffect> + 'a>| {
        command_handlers.push(Arc::clone(&operation));
        Step::operation(operation)
    };

    let mut steps = vec![
        step(Arc::new(ToolApprovalOperation::new(
            tool_inspection_manager,
        ))),
        step(Arc::new(SkillOperation::new(
            hook_manager.clone(),
            Arc::clone(&extension_lease),
        ))),
        step(Arc::new(RecipeOperation::new(
            provider.clone(),
            hook_manager.clone(),
        ))),
        step(Arc::new(ToolExecutionOperation::new(
            Arc::clone(&extension_manager),
            hook_manager.clone(),
            Arc::clone(&extension_lease),
        ))),
        step(Arc::new(RemainingToolCallsOperation::new(
            hook_manager.clone(),
        ))),
        step(Arc::new(SubagentOperation::new(subagent_runner.clone())))
            .gated_by([tool_calls_answered()]),
        step(Arc::new(SteerOperation::new(
            steer_queue,
            hook_manager.clone(),
        )))
        .gated_by(inference_ready()),
        step(Arc::new(BangShellOperation::new())),
    ];
    if !manages_own_context {
        steps.push(
            step(Arc::new(CompactionOperation::new(
                provider.clone(),
                model_config.clone(),
                context_limit,
                compaction_threshold,
            )))
            .gated_by(inference_ready()),
        );
    }
    steps.extend([
        step(Arc::new(ToolPairCompactionOperation::new(
            provider.clone(),
            model_config.clone(),
            tool_call_cutoff,
            tool_pair_summarization && !manages_own_context,
        ))),
        step(Arc::new(DoctorOperation::new(session_manager))),
        step(Arc::new(ProjectOperation)),
        step(Arc::new(RetryOperation::new(
            retry_timeout,
            on_failure_timeout,
        ))),
        step(Arc::new(EmptyResponseOperation)),
        step(Arc::new(StopHookOperation::new(
            hook_manager,
            stop_hook_block_cap,
        ))),
        step(Arc::new(ExitOnErrorOperation)),
        step(Arc::new(MaxTurnsOperation::new(max_turns))).gated_by(inference_ready()),
    ]);
    let request_preparer = GooseInferenceRequestPreparer {
        extension_manager,
        extension_lease,
        tool_inspection_manager,
        context_limit,
    };
    let status_operation = Arc::new(StatusOperation::new(provider.clone(), model_config.clone()));
    let inference_provider = Arc::new(GooseInferenceProvider::new(provider));
    let inference = Arc::new(
        InferenceRunner::new(inference_provider, model_config)
            .with_request_preparer(Arc::new(request_preparer)),
    );
    command_handlers.push(status_operation);
    let command_operation = Arc::new(SlashCommandOperation::new(command_handlers));
    let steps = std::iter::once(Step::operation(command_operation))
        .chain(steps)
        .chain(std::iter::once(
            Step::inference(inference).gated_by(inference_ready()),
        ))
        .collect();

    StateMachine::new(steps, cancel)
}
