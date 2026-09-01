use std::{
    borrow::Cow,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
};

use anyhow::Result;
use async_trait::async_trait;

use goose_agent::{
    operation::{ConversationEffect, Emitter, InferenceTools, Operation, OperationResult},
    tool::{ToolOperation, ToolProvider},
};
use goose_provider_types::conversation::{
    message::{Message, MessageContent},
    Conversation,
};
use rmcp::{
    handler::server::router::tool::{AsyncTool, SyncTool, ToolBase},
    model::{CallToolRequestParams, CallToolResult, ErrorData, Tool},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

#[derive(Default, Deserialize, JsonSchema)]
struct AddInput {
    left: u64,
    right: u64,
}

#[derive(Serialize, JsonSchema)]
struct AddOutput {
    sum: u64,
}

struct Add;

impl ToolBase for Add {
    type Parameter = AddInput;
    type Output = AddOutput;
    type Error = ErrorData;

    fn name() -> Cow<'static, str> {
        "add".into()
    }

    fn description() -> Option<Cow<'static, str>> {
        Some("Add two integers".into())
    }
}

impl SyncTool<()> for Add {
    fn invoke(_session: &(), input: AddInput) -> Result<AddOutput, ErrorData> {
        Ok(AddOutput {
            sum: input.left + input.right,
        })
    }
}

#[derive(Default, Deserialize, JsonSchema)]
struct GreetInput {
    name: String,
}

#[derive(Serialize, JsonSchema)]
struct GreetOutput {
    greeting: String,
}

struct Greet;

impl ToolBase for Greet {
    type Parameter = GreetInput;
    type Output = GreetOutput;
    type Error = ErrorData;

    fn name() -> Cow<'static, str> {
        "greet".into()
    }
}

impl AsyncTool<()> for Greet {
    async fn invoke(_session: &(), input: GreetInput) -> Result<GreetOutput, ErrorData> {
        Ok(GreetOutput {
            greeting: format!("Hello, {}!", input.name),
        })
    }
}

fn emitter() -> Emitter {
    emitter_with_token(CancellationToken::new())
}

fn emitter_with_token(cancel: CancellationToken) -> Emitter {
    let (tx, _rx) = mpsc::channel(1);
    Emitter::new(tx, cancel)
}

fn operation() -> ToolOperation<()> {
    ToolOperation::new()
        .with_sync_tool::<Add>()
        .with_async_tool::<Greet>()
}

fn with_tool_routes(mut message: Message, inference_tools: InferenceTools) -> Message {
    message.metadata.operations = Some(Box::new(std::collections::BTreeMap::from([(
        "tools".to_string(),
        inference_tools.message_notes,
    )])));
    message
}

#[tokio::test]
async fn advertises_user_defined_tools() {
    let operation = operation();
    let conversation = Conversation::new_unvalidated([Message::user().with_text("use tools")]);

    let tools = <ToolOperation<()> as Operation<(), ConversationEffect>>::inference_tools(
        &operation,
        &(),
        &conversation,
        &emitter(),
    )
    .await
    .unwrap()
    .tools;

    assert_eq!(tools.len(), 2);
    assert_eq!(tools[0].name, "add");
    assert_eq!(tools[0].description.as_deref(), Some("Add two integers"));
    assert!(tools[0].input_schema["properties"]["left"].is_object());
    assert_eq!(tools[1].name, "greet");
}

#[tokio::test]
async fn dispatches_calls_to_user_defined_tools() {
    let operation = operation();
    let conversation = Conversation::new_unvalidated(vec![
        Message::user().with_text("do both"),
        Message::assistant()
            .with_tool_request(
                "call-1",
                Ok(CallToolRequestParams::new("add").with_arguments(
                    serde_json::from_value(json!({"left": 2, "right": 3})).unwrap(),
                )),
            )
            .with_tool_request(
                "call-2",
                Ok(CallToolRequestParams::new("greet")
                    .with_arguments(serde_json::from_value(json!({"name": "Goose"})).unwrap())),
            ),
    ]);

    let result = <ToolOperation<()> as Operation<(), ConversationEffect>>::run(
        &operation,
        &(),
        &conversation,
        &emitter(),
    )
    .await
    .unwrap();

    let OperationResult::Applied(result) = result else {
        panic!("tool operation should apply");
    };
    let ConversationEffect::AppendMessage(message) = &result.effects[0] else {
        panic!("tool operation should append a response");
    };
    let responses: Vec<_> = message
        .content
        .iter()
        .map(|content| match content {
            MessageContent::ToolResponse(response) => response,
            _ => panic!("expected tool responses"),
        })
        .collect();
    assert_eq!(responses[0].id, "call-1");
    assert_eq!(
        responses[0]
            .tool_result
            .as_ref()
            .unwrap()
            .structured_content,
        Some(json!({"sum": 5}))
    );
    assert_eq!(responses[1].id, "call-2");
    assert_eq!(
        responses[1]
            .tool_result
            .as_ref()
            .unwrap()
            .structured_content,
        Some(json!({"greeting": "Hello, Goose!"}))
    );
}

struct DynamicTools;

#[async_trait]
impl ToolProvider<bool> for DynamicTools {
    async fn tools(&self, enabled: &bool) -> Result<Vec<Tool>> {
        Ok(if *enabled {
            vec![Tool::new(
                "dynamic",
                "A session-dependent tool",
                Arc::new(serde_json::from_value(json!({"type": "object"}))?),
            )]
        } else {
            Vec::new()
        })
    }

    async fn call(
        &self,
        _session: &bool,
        request_id: &str,
        call: CallToolRequestParams,
        _emit: &Emitter,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(CallToolResult::structured(json!({
            "request_id": request_id,
            "name": call.name
        })))
    }
}

struct ChangesAfterAdvertising {
    advertised: AtomicBool,
    discoveries: AtomicUsize,
}

#[async_trait]
impl ToolProvider<()> for ChangesAfterAdvertising {
    async fn tools(&self, _session: &()) -> Result<Vec<Tool>> {
        self.discoveries.fetch_add(1, Ordering::SeqCst);
        Ok(if self.advertised.swap(true, Ordering::SeqCst) {
            Vec::new()
        } else {
            vec![Tool::new(
                "transient",
                "Only discoverable once",
                Arc::new(serde_json::from_value(json!({"type": "object"}))?),
            )]
        })
    }

    async fn call(
        &self,
        _session: &(),
        _request_id: &str,
        _call: CallToolRequestParams,
        _emit: &Emitter,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(CallToolResult::structured(json!({"called": true})))
    }
}

struct NeverAdvertises;

#[async_trait]
impl ToolProvider<()> for NeverAdvertises {
    async fn tools(&self, _session: &()) -> Result<Vec<Tool>> {
        Ok(Vec::new())
    }

    async fn call(
        &self,
        _session: &(),
        _request_id: &str,
        _call: CallToolRequestParams,
        _emit: &Emitter,
    ) -> Result<CallToolResult, ErrorData> {
        panic!("the unrelated provider must not receive the persisted call")
    }
}

#[tokio::test]
async fn dispatches_from_persisted_routes_after_reconstructing_the_operation() {
    let provider = Arc::new(ChangesAfterAdvertising {
        advertised: AtomicBool::new(false),
        discoveries: AtomicUsize::new(0),
    });
    let operation = ToolOperation::new().with_provider("transient", provider.clone());
    let conversation = Conversation::new_unvalidated([
        Message::user().with_text("call it"),
        Message::assistant()
            .with_tool_request("call-1", Ok(CallToolRequestParams::new("transient"))),
    ]);
    let kickoff_result = <ToolOperation<()> as Operation<(), ConversationEffect>>::run(
        &operation,
        &(),
        &Conversation::new_unvalidated([Message::user().with_text("call it")]),
        &emitter(),
    )
    .await
    .unwrap();
    assert!(matches!(kickoff_result, OperationResult::NotApplicable));
    assert_eq!(provider.discoveries.load(Ordering::SeqCst), 0);

    let mut inference_tools =
        <ToolOperation<()> as Operation<(), ConversationEffect>>::inference_tools(
            &operation,
            &(),
            &conversation,
            &emitter(),
        )
        .await
        .unwrap();
    assert_eq!(inference_tools.tools[0].name, "transient");
    inference_tools
        .message_notes
        .get_mut("routes")
        .and_then(serde_json::Value::as_object_mut)
        .unwrap()
        .insert("unused".to_string(), json!("removed-provider"));

    let conversation = Conversation::new_unvalidated([
        Message::user().with_text("call it"),
        with_tool_routes(
            Message::assistant()
                .with_tool_request("call-1", Ok(CallToolRequestParams::new("transient"))),
            inference_tools,
        ),
    ]);
    let reconstructed = ToolOperation::new()
        .with_provider("unrelated", Arc::new(NeverAdvertises))
        .with_provider("transient", provider);
    let result = <ToolOperation<()> as Operation<(), ConversationEffect>>::run(
        &reconstructed,
        &(),
        &conversation,
        &emitter(),
    )
    .await
    .unwrap();

    let OperationResult::Applied(result) = result else {
        panic!("tool operation should apply");
    };
    let ConversationEffect::AppendMessage(message) = &result.effects[0] else {
        panic!("tool operation should append a response");
    };
    assert_eq!(
        message.content[0]
            .as_tool_response()
            .unwrap()
            .tool_result
            .as_ref()
            .unwrap()
            .structured_content,
        Some(json!({"called": true}))
    );
}

struct SessionRoutedTools {
    enabled_for: bool,
    calls: Arc<AtomicUsize>,
}

#[async_trait]
impl ToolProvider<bool> for SessionRoutedTools {
    async fn tools(&self, session: &bool) -> Result<Vec<Tool>> {
        Ok(if *session == self.enabled_for {
            vec![Tool::new(
                "routed",
                "A session-routed tool",
                Arc::new(serde_json::from_value(json!({"type": "object"}))?),
            )]
        } else {
            Vec::new()
        })
    }

    async fn call(
        &self,
        _session: &bool,
        _request_id: &str,
        _call: CallToolRequestParams,
        _emit: &Emitter,
    ) -> Result<CallToolResult, ErrorData> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(CallToolResult::structured(json!({"called": true})))
    }
}

#[tokio::test]
async fn advertised_routes_are_isolated_by_turn() {
    let false_calls = Arc::new(AtomicUsize::new(0));
    let true_calls = Arc::new(AtomicUsize::new(0));
    let operation = ToolOperation::new()
        .with_provider(
            "false",
            Arc::new(SessionRoutedTools {
                enabled_for: false,
                calls: false_calls.clone(),
            }),
        )
        .with_provider(
            "true",
            Arc::new(SessionRoutedTools {
                enabled_for: true,
                calls: true_calls.clone(),
            }),
        );
    let false_conversation = Conversation::new_unvalidated([
        Message::user().with_text("false session"),
        Message::assistant()
            .with_tool_request("false-call", Ok(CallToolRequestParams::new("routed"))),
    ]);
    let true_conversation = Conversation::new_unvalidated([
        Message::user().with_text("true session"),
        Message::assistant()
            .with_tool_request("true-call", Ok(CallToolRequestParams::new("routed"))),
    ]);

    let false_tools =
        <ToolOperation<bool> as Operation<bool, ConversationEffect>>::inference_tools(
            &operation,
            &false,
            &false_conversation,
            &emitter(),
        )
        .await
        .unwrap();
    let true_tools = <ToolOperation<bool> as Operation<bool, ConversationEffect>>::inference_tools(
        &operation,
        &true,
        &true_conversation,
        &emitter(),
    )
    .await
    .unwrap();
    let false_conversation = Conversation::new_unvalidated([
        Message::user().with_text("false session"),
        with_tool_routes(
            Message::assistant()
                .with_tool_request("false-call", Ok(CallToolRequestParams::new("routed"))),
            false_tools,
        ),
    ]);
    let true_conversation = Conversation::new_unvalidated([
        Message::user().with_text("true session"),
        with_tool_routes(
            Message::assistant()
                .with_tool_request("true-call", Ok(CallToolRequestParams::new("routed"))),
            true_tools,
        ),
    ]);
    <ToolOperation<bool> as Operation<bool, ConversationEffect>>::run(
        &operation,
        &false,
        &false_conversation,
        &emitter(),
    )
    .await
    .unwrap();
    <ToolOperation<bool> as Operation<bool, ConversationEffect>>::run(
        &operation,
        &true,
        &true_conversation,
        &emitter(),
    )
    .await
    .unwrap();

    assert_eq!(false_calls.load(Ordering::SeqCst), 1);
    assert_eq!(true_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn discovers_and_dispatches_dynamic_tools_per_session() {
    let operation = ToolOperation::new().with_provider("dynamic", Arc::new(DynamicTools));
    let conversation = Conversation::new_unvalidated([
        Message::user().with_text("call the dynamic tool"),
        Message::assistant()
            .with_tool_request("dynamic-call", Ok(CallToolRequestParams::new("dynamic"))),
    ]);
    let disabled_conversation =
        Conversation::new_unvalidated([Message::user().with_text("no tools")]);
    let enabled_tools =
        <ToolOperation<bool> as Operation<bool, ConversationEffect>>::inference_tools(
            &operation,
            &true,
            &conversation,
            &emitter(),
        )
        .await
        .unwrap()
        .tools;
    let disabled_tools =
        <ToolOperation<bool> as Operation<bool, ConversationEffect>>::inference_tools(
            &operation,
            &false,
            &disabled_conversation,
            &emitter(),
        )
        .await
        .unwrap()
        .tools;
    assert_eq!(enabled_tools[0].name, "dynamic");
    assert!(disabled_tools.is_empty());

    let conversation = Conversation::new_unvalidated([
        Message::user().with_text("call the dynamic tool"),
        Message::assistant()
            .with_tool_request("dynamic-call", Ok(CallToolRequestParams::new("dynamic"))),
    ]);
    let result = <ToolOperation<bool> as Operation<bool, ConversationEffect>>::run(
        &operation,
        &true,
        &conversation,
        &emitter(),
    )
    .await
    .unwrap();
    let OperationResult::Applied(result) = result else {
        panic!("dynamic tool operation should apply");
    };
    let ConversationEffect::AppendMessage(message) = &result.effects[0] else {
        panic!("dynamic tool operation should append a response");
    };
    let response = message.content[0].as_tool_response().unwrap();
    assert_eq!(
        response.tool_result.as_ref().unwrap().structured_content,
        Some(json!({"request_id": "dynamic-call", "name": "dynamic"}))
    );
}

#[tokio::test]
async fn ignores_tool_requests_from_an_earlier_turn() {
    let operation = operation();
    let conversation = Conversation::new_unvalidated([
        Message::user().with_text("old turn"),
        Message::assistant().with_tool_request(
            "stale-call",
            Ok(CallToolRequestParams::new("add")
                .with_arguments(serde_json::from_value(json!({"left": 2, "right": 3})).unwrap())),
        ),
        Message::user().with_text("new turn"),
    ]);

    let result = <ToolOperation<()> as Operation<(), ConversationEffect>>::run(
        &operation,
        &(),
        &conversation,
        &emitter(),
    )
    .await
    .unwrap();

    assert!(matches!(result, OperationResult::NotApplicable));
}

#[tokio::test]
async fn rejects_duplicate_dynamic_tool_names() {
    let operation = ToolOperation::new()
        .with_provider("first", Arc::new(DynamicTools))
        .with_provider("second", Arc::new(DynamicTools));

    let conversation = Conversation::new_unvalidated([Message::user().with_text("use tools")]);
    let error = <ToolOperation<bool> as Operation<bool, ConversationEffect>>::inference_tools(
        &operation,
        &true,
        &conversation,
        &emitter(),
    )
    .await
    .unwrap_err();

    assert_eq!(
        error.to_string(),
        "multiple tool providers registered 'dynamic'"
    );
}

#[tokio::test]
async fn responds_to_unparseable_tool_requests() {
    let operation = operation();
    let conversation = Conversation::new_unvalidated([
        Message::user().with_text("call a tool"),
        Message::assistant().with_tool_request(
            "invalid-call",
            Err(ErrorData::invalid_params("malformed arguments", None)),
        ),
    ]);

    let result = <ToolOperation<()> as Operation<(), ConversationEffect>>::run(
        &operation,
        &(),
        &conversation,
        &emitter(),
    )
    .await
    .unwrap();

    let OperationResult::Applied(result) = result else {
        panic!("tool operation should apply");
    };
    let ConversationEffect::AppendMessage(message) = &result.effects[0] else {
        panic!("tool operation should append a response");
    };
    let response = message.content[0].as_tool_response().unwrap();
    assert_eq!(response.id, "invalid-call");
    assert_eq!(
        response.tool_result.as_ref().unwrap_err().message,
        "malformed arguments"
    );
}

#[derive(Default, Clone)]
struct BlockingSession {
    started: Arc<AtomicBool>,
}

struct BlockingSyncTool;

impl ToolBase for BlockingSyncTool {
    type Parameter = ();
    type Output = ();
    type Error = ErrorData;

    fn name() -> Cow<'static, str> {
        "blocking_sync".into()
    }

    fn input_schema() -> Option<Arc<serde_json::Map<String, serde_json::Value>>> {
        None
    }
}

impl SyncTool<BlockingSession> for BlockingSyncTool {
    fn invoke(session: &BlockingSession, _input: ()) -> Result<(), ErrorData> {
        session.started.store(true, Ordering::SeqCst);
        std::thread::sleep(std::time::Duration::from_millis(100));
        Ok(())
    }
}

#[tokio::test]
async fn cancellation_interrupts_blocking_sync_tools() {
    let session = BlockingSession::default();
    let started = session.started.clone();
    let operation = ToolOperation::new().with_sync_tool::<BlockingSyncTool>();
    let conversation = Conversation::new_unvalidated([
        Message::user().with_text("call it"),
        Message::assistant()
            .with_tool_request("call-1", Ok(CallToolRequestParams::new("blocking_sync"))),
    ]);
    let cancel = CancellationToken::new();
    let run_cancel = cancel.clone();
    let run = tokio::spawn(async move {
        <ToolOperation<BlockingSession> as Operation<BlockingSession, ConversationEffect>>::run(
            &operation,
            &session,
            &conversation,
            &emitter_with_token(run_cancel),
        )
        .await
    });
    while !started.load(Ordering::SeqCst) {
        tokio::task::yield_now().await;
    }
    cancel.cancel();
    let result = tokio::time::timeout(std::time::Duration::from_millis(50), run)
        .await
        .expect("operation should not wait for the blocking tool")
        .unwrap()
        .unwrap();

    let OperationResult::Applied(result) = result else {
        panic!("tool operation should apply");
    };
    let ConversationEffect::AppendMessage(message) = &result.effects[0] else {
        panic!("tool operation should append a response");
    };
    let response = message.content[0].as_tool_response().unwrap();
    assert!(response
        .tool_result
        .as_ref()
        .unwrap()
        .is_error
        .is_some_and(|is_error| is_error));
}

struct BlockingTools {
    calls: Arc<AtomicUsize>,
}

#[async_trait]
impl ToolProvider<()> for BlockingTools {
    async fn tools(&self, _session: &()) -> Result<Vec<Tool>> {
        Ok(vec![Tool::new(
            "blocking",
            "A tool that never finishes",
            Arc::new(serde_json::from_value(json!({"type": "object"}))?),
        )])
    }

    async fn call(
        &self,
        _session: &(),
        _request_id: &str,
        _call: CallToolRequestParams,
        _emit: &Emitter,
    ) -> Result<CallToolResult, ErrorData> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        std::future::pending().await
    }
}

#[tokio::test]
async fn cancellation_interrupts_current_and_remaining_calls() {
    let calls = Arc::new(AtomicUsize::new(0));
    let operation = ToolOperation::new().with_provider(
        "blocking",
        Arc::new(BlockingTools {
            calls: calls.clone(),
        }),
    );
    let conversation = Conversation::new_unvalidated([
        Message::user().with_text("call twice"),
        Message::assistant()
            .with_tool_request("call-1", Ok(CallToolRequestParams::new("blocking")))
            .with_tool_request("call-2", Ok(CallToolRequestParams::new("blocking"))),
    ]);
    let cancel = CancellationToken::new();
    let run_cancel = cancel.clone();
    let run = tokio::spawn(async move {
        <ToolOperation<()> as Operation<(), ConversationEffect>>::run(
            &operation,
            &(),
            &conversation,
            &emitter_with_token(run_cancel),
        )
        .await
    });
    while calls.load(Ordering::SeqCst) == 0 {
        tokio::task::yield_now().await;
    }
    cancel.cancel();
    let result = run.await.unwrap().unwrap();

    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let OperationResult::Applied(result) = result else {
        panic!("tool operation should apply");
    };
    let ConversationEffect::AppendMessage(message) = &result.effects[0] else {
        panic!("tool operation should append a response");
    };
    assert_eq!(
        message.get_tool_response_ids(),
        ["call-1", "call-2"].into_iter().collect()
    );
    for content in &message.content {
        let response = content.as_tool_response().unwrap();
        let result = response.tool_result.as_ref().unwrap();
        assert!(result.is_error.is_some_and(|is_error| is_error));
        assert_eq!(
            result.content[0].as_text().unwrap().text,
            "Tool call was interrupted before completing"
        );
    }
}

#[tokio::test]
async fn ignores_unregistered_and_answered_requests() {
    let operation = operation();
    let conversation = Conversation::new_unvalidated(vec![
        Message::user().with_text("call a tool"),
        Message::assistant()
            .with_tool_request("call-1", Ok(CallToolRequestParams::new("add")))
            .with_tool_request("call-2", Ok(CallToolRequestParams::new("not_registered"))),
        Message::user().with_tool_response(
            "call-1",
            Ok(rmcp::model::CallToolResult::structured(json!({"sum": 5}))),
        ),
    ]);

    let result = <ToolOperation<()> as Operation<(), ConversationEffect>>::run(
        &operation,
        &(),
        &conversation,
        &emitter(),
    )
    .await
    .unwrap();

    assert!(matches!(result, OperationResult::NotApplicable));
}

struct BlockingDiscovery;

#[async_trait]
impl ToolProvider<()> for BlockingDiscovery {
    async fn tools(&self, _session: &()) -> Result<Vec<Tool>> {
        std::future::pending().await
    }

    async fn call(
        &self,
        _session: &(),
        _request_id: &str,
        _call: CallToolRequestParams,
        _emit: &Emitter,
    ) -> Result<CallToolResult, ErrorData> {
        unreachable!()
    }
}

#[tokio::test]
async fn cancellation_interrupts_dynamic_tool_discovery() {
    let operation =
        ToolOperation::new().with_provider("blocking-discovery", Arc::new(BlockingDiscovery));
    let conversation = Conversation::new_unvalidated([Message::user().with_text("use tools")]);
    let cancel = CancellationToken::new();
    cancel.cancel();

    let error = tokio::time::timeout(
        std::time::Duration::from_millis(50),
        <ToolOperation<()> as Operation<(), ConversationEffect>>::inference_tools(
            &operation,
            &(),
            &conversation,
            &emitter_with_token(cancel),
        ),
    )
    .await
    .expect("tool discovery should observe cancellation")
    .unwrap_err();

    assert_eq!(error.to_string(), "tool discovery cancelled");
}

#[tokio::test]
async fn skips_externally_dispatched_tool_requests() {
    let operation = operation();
    let conversation = Conversation::new_unvalidated([
        Message::user().with_text("call a tool"),
        Message::assistant().with_tool_request_with_metadata(
            "external-call",
            Ok(CallToolRequestParams::new("add")),
            None,
            Some(json!({"goose.external_dispatch": true})),
        ),
    ]);

    let result = <ToolOperation<()> as Operation<(), ConversationEffect>>::run(
        &operation,
        &(),
        &conversation,
        &emitter(),
    )
    .await
    .unwrap();

    assert!(matches!(result, OperationResult::NotApplicable));
}
