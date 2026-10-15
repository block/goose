use std::sync::{Arc, Mutex};

use anyhow::Result;
use async_trait::async_trait;
use goose_agent::{
    machine::{EffectHandler, MachineSession, SessionLoader, StateMachine, Step},
    operation::{
        applied, not_applicable, yielded, ConversationEffect, Emitter, Inference, InferenceInput,
        Operation, OperationResult, OperationStream,
    },
    step_gate::{subagents_finished, tool_calls_answered, StepGate},
    subagent::{SubagentRunner, SubagentUpdate},
};
use goose_provider_types::conversation::{
    message::{Message, TOOL_META_EXTERNAL_DISPATCH_KEY},
    Conversation,
};
use rmcp::model::{CallToolRequestParams, CallToolResult, Role, Tool};
use serde_json::json;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
struct Session(Conversation);

impl MachineSession for Session {
    fn id(&self) -> &str {
        "session"
    }

    fn conversation(&self) -> Option<&Conversation> {
        Some(&self.0)
    }
}

struct Runtime(Mutex<Session>);

impl Runtime {
    fn new(messages: impl IntoIterator<Item = Message>) -> Self {
        Self(Mutex::new(Session(Conversation::new_unvalidated(messages))))
    }
}

#[async_trait]
impl SessionLoader<Session> for Runtime {
    async fn load(&self, _session_id: &str) -> Result<Session> {
        Ok(self.0.lock().unwrap().clone())
    }
}

#[async_trait]
impl EffectHandler<Session, ConversationEffect> for Runtime {
    async fn apply_effects(
        &self,
        _session: &Session,
        effects: &mut [ConversationEffect],
        _emit: &Emitter,
    ) -> Result<()> {
        for effect in effects {
            let ConversationEffect::AppendMessage(message) = effect else {
                panic!("unexpected effect");
            };
            self.0.lock().unwrap().0.push(message.clone());
        }
        Ok(())
    }
}

fn texts(session: &Session) -> Vec<String> {
    session.0.iter().map(Message::as_concat_text).collect()
}

fn emitter(cancel: CancellationToken) -> Emitter {
    let (tx, _rx) = mpsc::unbounded_channel();
    Emitter::new(tx, cancel)
}

fn is_prepared(conversation: &Conversation) -> bool {
    conversation
        .iter()
        .any(|message| message.as_concat_text() == "prepared")
}

fn prepared() -> StepGate<'static, Session> {
    StepGate::new("prepared", |_session, conversation| {
        is_prepared(conversation)
    })
}

fn closed() -> StepGate<'static, Session> {
    StepGate::new("closed", |_session, _conversation| false)
}

struct Prepare;

#[async_trait]
impl Operation<Session> for Prepare {
    fn name(&self) -> &'static str {
        "prepare"
    }

    async fn run(
        &self,
        _session: &Session,
        conversation: &Conversation,
        _emit: &Emitter,
    ) -> Result<OperationResult> {
        if is_prepared(conversation) {
            return not_applicable();
        }
        applied([Message::user().with_text("prepared").into()])
    }
}

struct Reply;

#[async_trait]
impl Operation<Session> for Reply {
    fn name(&self) -> &'static str {
        "reply"
    }

    async fn run(
        &self,
        _session: &Session,
        conversation: &Conversation,
        _emit: &Emitter,
    ) -> Result<OperationResult> {
        if conversation
            .iter()
            .any(|message| message.role == Role::Assistant)
        {
            return not_applicable();
        }
        applied([Message::assistant().with_text("reply").into()])
    }
}

#[tokio::test]
async fn a_gated_step_waits_for_its_gate_even_when_listed_first() -> Result<()> {
    let cancel = CancellationToken::new();
    let machine = StateMachine::new(
        vec![
            Step::operation(Arc::new(Reply)).gated_by([prepared()]),
            Step::operation(Arc::new(Prepare)),
        ],
        cancel.clone(),
    );
    let runtime = Runtime::new([Message::user().with_text("kickoff")]);

    let session = machine.run(&runtime, "session", &emitter(cancel)).await?;

    assert_eq!(texts(&session), ["kickoff", "prepared", "reply"]);
    Ok(())
}

struct Contributor;

#[async_trait]
impl Operation<Session> for Contributor {
    fn name(&self) -> &'static str {
        "contributor"
    }

    async fn inference_tools(&self, _session: &Session) -> Result<Vec<Tool>> {
        Ok(vec![Tool::new(
            "contributed",
            "a tool",
            Arc::new(serde_json::Map::new()),
        )])
    }

    async fn prompt_parts(
        &self,
        _session: &Session,
        _conversation: &Conversation,
    ) -> Result<Vec<(String, String)>> {
        Ok(vec![("contributed".into(), "a prompt part".into())])
    }

    async fn finalize_cancellation(
        &self,
        _session: &Session,
        _conversation: &Conversation,
        _emit: &Emitter,
    ) -> Vec<ConversationEffect> {
        vec![Message::assistant().with_text("cleaned up").into()]
    }

    async fn run(
        &self,
        _session: &Session,
        _conversation: &Conversation,
        _emit: &Emitter,
    ) -> Result<OperationResult> {
        panic!("a step behind a closed gate must not run");
    }
}

#[derive(Default)]
struct RecordingInference {
    input: Mutex<Option<(Vec<String>, Vec<String>)>>,
}

#[async_trait]
impl Operation<Session> for RecordingInference {
    fn name(&self) -> &'static str {
        "inference"
    }
}

#[async_trait]
impl Inference<Session> for RecordingInference {
    fn applies(&self, _conversation: &Conversation) -> bool {
        true
    }

    async fn infer(
        &self,
        _session: &Session,
        _conversation: &Conversation,
        input: InferenceInput,
        _emit: &Emitter,
    ) -> Result<OperationResult> {
        let tools = input
            .tools
            .iter()
            .map(|tool| tool.name.to_string())
            .collect();
        let parts = input.prompt_parts.into_iter().map(|(key, _)| key).collect();
        *self.input.lock().unwrap() = Some((tools, parts));
        yielded()
    }
}

#[tokio::test]
async fn a_gated_step_still_contributes_to_inference() -> Result<()> {
    let inference = Arc::new(RecordingInference::default());
    let cancel = CancellationToken::new();
    let machine = StateMachine::new(
        vec![
            Step::operation(Arc::new(Contributor)).gated_by([closed()]),
            Step::inference(inference.clone()),
        ],
        cancel.clone(),
    );
    let session = Session(Conversation::new_unvalidated([
        Message::user().with_text("kickoff")
    ]));

    machine.step(&session, &emitter(cancel)).await?;

    let input = inference.input.lock().unwrap().take();
    assert_eq!(
        input,
        Some((vec!["contributed".into()], vec!["contributed".into()]))
    );
    Ok(())
}

struct Stop;

#[async_trait]
impl Operation<Session> for Stop {
    fn name(&self) -> &'static str {
        "stop"
    }

    async fn run(
        &self,
        _session: &Session,
        _conversation: &Conversation,
        emit: &Emitter,
    ) -> Result<OperationResult> {
        emit.cancel_token().cancel();
        applied([])
    }
}

#[tokio::test]
async fn a_gated_step_still_cleans_up_on_cancellation() -> Result<()> {
    let cancel = CancellationToken::new();
    let machine = StateMachine::new(
        vec![
            Step::operation(Arc::new(Contributor)).gated_by([closed()]),
            Step::operation(Arc::new(Stop)),
        ],
        cancel.clone(),
    );
    let runtime = Runtime::new([Message::user().with_text("kickoff")]);

    let session = machine.run(&runtime, "session", &emitter(cancel)).await?;

    assert_eq!(texts(&session), ["kickoff", "cleaned up"]);
    Ok(())
}

struct Act;

#[async_trait]
impl Operation<Session> for Act {
    fn name(&self) -> &'static str {
        "act"
    }

    async fn run(
        &self,
        _session: &Session,
        _conversation: &Conversation,
        _emit: &Emitter,
    ) -> Result<OperationResult> {
        applied([])
    }
}

fn calls(ids: &[&str]) -> Message {
    ids.iter().fold(Message::assistant(), |message, id| {
        message.with_tool_request(*id, Ok(CallToolRequestParams::new("any_tool")))
    })
}

fn answer(id: &str) -> Message {
    Message::user().with_tool_response(id, Ok(CallToolResult::success(Vec::new())))
}

#[tokio::test]
async fn tool_calls_answered_is_closed_only_while_this_turn_has_an_unanswered_call() -> Result<()> {
    let kickoff = || Message::user().with_text("kickoff");
    let external = Message::assistant().with_tool_request_with_metadata(
        "external",
        Ok(CallToolRequestParams::new("any_tool")),
        None,
        Some(json!({ TOOL_META_EXTERNAL_DISPATCH_KEY: true })),
    );
    let cases = [
        ("no calls", vec![kickoff()], true),
        (
            "an unanswered call",
            vec![kickoff(), calls(&["open"])],
            false,
        ),
        (
            "a partly answered batch",
            vec![
                kickoff(),
                calls(&["allowed", "needs_approval"]),
                answer("allowed"),
            ],
            false,
        ),
        (
            "every call answered",
            vec![kickoff(), calls(&["done"]), answer("done")],
            true,
        ),
        (
            "an externally executed call",
            vec![kickoff(), external],
            true,
        ),
        (
            "an unanswered call from an earlier turn",
            vec![kickoff(), calls(&["stale"]), kickoff()],
            true,
        ),
    ];
    let cancel = CancellationToken::new();
    let machine = StateMachine::new(
        vec![Step::operation(Arc::new(Act)).gated_by([tool_calls_answered()])],
        cancel.clone(),
    );
    let emit = emitter(cancel);

    for (case, messages, open) in cases {
        let session = Session(Conversation::new_unvalidated(messages));

        let result = machine.step(&session, &emit).await?;

        assert_eq!(result.applied_step.is_some(), open, "{case}");
    }
    Ok(())
}

struct Started(Vec<&'static str>);

#[async_trait]
impl SubagentRunner<Session> for Started {
    fn started_subagent_session_ids(
        &self,
        _parent_session: &Session,
        _conversation: &Conversation,
    ) -> Vec<String> {
        self.0.iter().map(|id| id.to_string()).collect()
    }

    async fn start(
        &self,
        _parent_session: &Session,
        _subagent_session_id: &str,
        _cancel: CancellationToken,
    ) -> OperationStream<'static, SubagentUpdate> {
        panic!("a gate must not start subagents");
    }
}

fn subagent_note(key: &str, value: serde_json::Value) -> Message {
    let mut message = Message::user().with_visibility(false, true);
    message
        .metadata
        .set_operation_note("foreground_subagent", key, value);
    message
}

#[tokio::test]
async fn subagents_finished_is_closed_while_a_started_subagent_is_undelivered() -> Result<()> {
    let kickoff = || Message::user().with_text("kickoff");
    let delivered = || subagent_note("delivered", json!("a"));
    let cancelled = || subagent_note("cancelled", json!(["a"]));
    let cases = [
        ("none started", vec![], vec![kickoff()], true),
        ("one running", vec!["a"], vec![kickoff()], false),
        ("delivered", vec!["a"], vec![kickoff(), delivered()], true),
        ("cancelled", vec!["a"], vec![kickoff(), cancelled()], true),
        (
            "one delivered, one running",
            vec!["a", "b"],
            vec![kickoff(), delivered()],
            false,
        ),
    ];
    let cancel = CancellationToken::new();
    let emit = emitter(cancel.clone());

    for (case, started, messages, open) in cases {
        let machine = StateMachine::new(
            vec![Step::operation(Arc::new(Act))
                .gated_by([subagents_finished(Arc::new(Started(started)))])],
            cancel.clone(),
        );
        let session = Session(Conversation::new_unvalidated(messages));

        let result = machine.step(&session, &emit).await?;

        assert_eq!(result.applied_step.is_some(), open, "{case}");
    }
    Ok(())
}
