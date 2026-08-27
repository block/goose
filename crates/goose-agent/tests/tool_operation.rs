use std::borrow::Cow;

use goose_agent::{
    operation::{ConversationEffect, Emitter, Operation, OperationResult},
    tool::SyncToolOperation,
};
use goose_provider_types::conversation::{
    message::{Message, MessageContent},
    Conversation,
};
use rmcp::{
    handler::server::router::tool::{SyncTool, ToolBase},
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

fn emitter() -> Emitter {
    let (tx, _rx) = mpsc::channel(1);
    Emitter::new(tx, CancellationToken::new())
}

#[tokio::test]
async fn advertises_rmcp_tool_definition() {
    let operation = SyncToolOperation::<Add>::new();

    let tools = <SyncToolOperation<Add> as Operation<(), ConversationEffect>>::inference_tools(
        &operation,
        &(),
    )
    .await
    .unwrap();

    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "add");
    assert_eq!(tools[0].description.as_deref(), Some("Add two integers"));
    assert!(tools[0].input_schema["properties"]["left"].is_object());
}

#[tokio::test]
async fn executes_pending_tool_request() {
    let operation = SyncToolOperation::<Add>::new();
    let conversation = Conversation::new_unvalidated(vec![
        Message::user().with_text("add these"),
        Message::assistant().with_tool_request(
            "call-1",
            Ok(CallToolRequestParams::new("add")
                .with_arguments(serde_json::from_value(json!({"left": 2, "right": 3})).unwrap())),
        ),
    ]);

    let result = <SyncToolOperation<Add> as Operation<(), ConversationEffect>>::run(
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
    let MessageContent::ToolResponse(response) = &message.content[0] else {
        panic!("expected a tool response");
    };
    assert_eq!(response.id, "call-1");
    assert_eq!(
        response.tool_result.as_ref().unwrap().structured_content,
        Some(json!({"sum": 5}))
    );
}

#[tokio::test]
async fn does_not_execute_answered_request_twice() {
    let operation = SyncToolOperation::<Add>::new();
    let conversation = Conversation::new_unvalidated(vec![
        Message::assistant().with_tool_request("call-1", Ok(CallToolRequestParams::new("add"))),
        Message::user().with_tool_response(
            "call-1",
            Ok(rmcp::model::CallToolResult::structured(json!({"sum": 5}))),
        ),
    ]);

    let result = <SyncToolOperation<Add> as Operation<(), ConversationEffect>>::run(
        &operation,
        &(),
        &conversation,
        &emitter(),
    )
    .await
    .unwrap();

    assert!(matches!(result, OperationResult::NotApplicable));
}
