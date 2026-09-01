use std::{collections::HashSet, sync::Arc};

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use crate::operation::{
    ConversationEffect, Emitter, Inference, InferenceInput, MachineEffect, Operation,
    OperationFuture, OperationResult, StepResult,
};
use goose_provider_types::conversation::Conversation;

pub trait MachineSession: Send + Sync {
    fn id(&self) -> &str;
    fn conversation(&self) -> Option<&Conversation>;
}

#[async_trait]
pub trait SessionLoader<S>: Send + Sync {
    async fn load(&self, session_id: &str) -> Result<S>;
}

#[async_trait]
pub trait EffectHandler<S, E>: Send + Sync {
    async fn apply_effects(&self, session: &S, effects: &mut [E], emit: &Emitter) -> Result<()>;
}

pub trait EffectUsage<E>: Send + Sync {
    fn usage(&self, _effect: &E) -> Option<goose_provider_types::conversation::token_usage::Usage> {
        None
    }
}

pub enum Step<'a, S, E = ConversationEffect> {
    Operation(Arc<dyn Operation<S, E> + 'a>),
    Inference(Arc<dyn Inference<S, E> + 'a>),
}

impl<S, E: Send> Step<'_, S, E> {
    fn operation(&self) -> &dyn Operation<S, E> {
        match self {
            Step::Operation(operation) => operation.as_ref(),
            Step::Inference(inference) => inference.as_ref(),
        }
    }
}

pub struct StateMachine<'a, S, E = ConversationEffect> {
    steps: Vec<Step<'a, S, E>>,
    cancel: CancellationToken,
}

fn add_inference_tools(
    input: &mut InferenceInput,
    tool_names: &mut HashSet<String>,
    operation_name: &str,
    inference_tools: crate::operation::InferenceTools,
) -> Result<()> {
    for tool in inference_tools.tools {
        if !tool_names.insert(tool.name.to_string()) {
            anyhow::bail!("multiple operations registered tool '{}'", tool.name);
        }
        input.tools.push(tool);
    }
    if !inference_tools.message_notes.is_empty()
        && input
            .message_notes
            .insert(operation_name.to_string(), inference_tools.message_notes)
            .is_some()
    {
        anyhow::bail!("multiple operations named '{operation_name}' produced inference notes");
    }
    Ok(())
}

impl<'a, S, E> StateMachine<'a, S, E>
where
    S: MachineSession,
    E: MachineEffect + Send + 'static,
{
    pub fn new(steps: Vec<Step<'a, S, E>>, cancel: CancellationToken) -> Self {
        Self { steps, cancel }
    }

    pub async fn step(&self, session: &S, emit: &Emitter) -> Result<Option<StepResult<E>>> {
        let conversation = session
            .conversation()
            .ok_or_else(|| anyhow!("state-machine session loaded without conversation"))?;

        for step in &self.steps {
            let name = step.operation().name();
            let result = if self.cancel.is_cancelled() {
                OperationResult::NotApplicable
            } else {
                let step_fut: OperationFuture<'_, Result<OperationResult<E>>> = match step {
                    Step::Operation(operation) => operation.run(session, conversation, emit),
                    Step::Inference(inference) => {
                        let mut input = InferenceInput::default();
                        let mut tool_names = HashSet::new();
                        for operation in self.steps.iter().map(|step| step.operation()) {
                            add_inference_tools(
                                &mut input,
                                &mut tool_names,
                                operation.name(),
                                operation.inference_tools(session, conversation).await?,
                            )?;
                            input
                                .prompt_parts
                                .extend(operation.prompt_parts(session, conversation).await?);
                            input
                                .moim_parts
                                .extend(operation.moim_parts(session, conversation).await?);
                        }
                        inference.infer(session, conversation, input, emit)
                    }
                };
                step_fut.await?
            };
            let cancelled = self.cancel.is_cancelled();
            let result = if cancelled {
                step.operation()
                    .cancel(session, conversation, result, emit)
                    .await?
            } else {
                result
            };

            match result {
                OperationResult::NotApplicable => {}
                OperationResult::Applied(mut result) => {
                    result.applied_step = Some(name);
                    for effect in &mut result.effects {
                        effect.ensure_message_ids();
                    }
                    if cancelled {
                        result.yield_to_client = true;
                    }
                    return Ok(Some(result));
                }
            }
        }

        Ok(None)
    }

    pub async fn apply<R>(
        &self,
        runtime: &R,
        session: &S,
        result: &mut StepResult<E>,
        emit: &Emitter,
    ) -> Result<()>
    where
        R: EffectHandler<S, E>,
    {
        for effect in &mut result.effects {
            effect.ensure_message_ids();
        }
        runtime
            .apply_effects(session, &mut result.effects, emit)
            .await
    }

    pub async fn run<R>(&self, runtime: &R, session_id: &str, emit: &Emitter) -> Result<S>
    where
        R: SessionLoader<S> + EffectHandler<S, E>,
    {
        loop {
            let session = runtime.load(session_id).await?;
            let Some(mut result) = self.step(&session, emit).await? else {
                break;
            };
            self.apply(runtime, &session, &mut result, emit).await?;
            if result.yield_to_client {
                break;
            }
        }
        runtime.load(session_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_duplicate_tools_across_operations() {
        let schema = Arc::new(serde_json::Map::new());
        let mut input = InferenceInput::default();
        let mut names = HashSet::new();

        add_inference_tools(
            &mut input,
            &mut names,
            "first",
            crate::operation::InferenceTools {
                tools: vec![rmcp::model::Tool::new("duplicate", "first", schema.clone())],
                ..Default::default()
            },
        )
        .unwrap();
        let error = add_inference_tools(
            &mut input,
            &mut names,
            "second",
            crate::operation::InferenceTools {
                tools: vec![rmcp::model::Tool::new("duplicate", "second", schema)],
                ..Default::default()
            },
        )
        .unwrap_err();

        assert_eq!(
            error.to_string(),
            "multiple operations registered tool 'duplicate'"
        );
    }
}
