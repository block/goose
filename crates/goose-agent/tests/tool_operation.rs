use std::borrow::Cow;

use goose_agent::{
    operation::{ConversationEffect, Emitter, Operation, OperationResult},
    tool::ToolOperation,
};
use goose_provider_types::conversation::{
    message::{Message, MessageContent},
    Conversation,
};
use rmcp::{
    handler::server::router::tool::{AsyncTool, SyncTool, ToolBase},
    model::{CallToolRequestParams, ErrorData},
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
    let (tx, _rx) = mpsc::channel(1);
    Emitter::new(tx, CancellationToken::new())
}

fn operation() -> ToolOperation<()> {
    ToolOperation::new()
        .with_sync_tool::<Add>()
        .with_async_tool::<Greet>()
}

#[tokio::test]
async fn advertises_user_defined_tools() {
    let operation = operation();

    let tools =
        <ToolOperation<()> as Operation<(), ConversationEffect>>::inference_tools(&operation, &())
            .await
            .unwrap();

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

#[tokio::test]
async fn ignores_unregistered_and_answered_requests() {
    let operation = operation();
    let conversation = Conversation::new_unvalidated(vec![
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
