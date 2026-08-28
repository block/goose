//! Covers `Agent::reply_with_state_machine`, the entry point the CLI and desktop
//! reach when the state machine is enabled.

use std::sync::Arc;
use std::time::Duration;

use agent_client_protocol::schema::v1::{
    Annotations as AcpAnnotations, ContentBlock as AcpContentBlock, EmbeddedResource,
    EmbeddedResourceResource, ResourceLink, Role as AcpRole, TextContent as AcpTextContent,
    TextResourceContents,
};
use anyhow::Result;
use futures::StreamExt;
use tokio_util::sync::CancellationToken;

use super::dummy_api::{DummyApi, ProviderFeatures};
use crate::acp::server::GooseAcpAgent;
use crate::agents::extension::ExtensionConfig;
use crate::agents::final_output_tool::FINAL_OUTPUT_TOOL_NAME;
use crate::agents::{Agent, AgentConfig, AgentEvent, GoosePlatform, SessionConfig};
use crate::config::permission::PermissionManager;
use crate::config::GooseMode;
use crate::conversation::message::{Message, MessageContent};
use crate::providers::base::Provider;
use crate::session::{SessionManager, SessionType};
use goose_providers::model::ModelConfig;

async fn agent_with_dummy_api() -> Result<(Agent, Arc<DummyApi>, String, tempfile::TempDir)> {
    agent_with_named_dummy_api("openai").await
}

async fn agent_with_named_dummy_api(
    provider_name: &str,
) -> Result<(Agent, Arc<DummyApi>, String, tempfile::TempDir)> {
    let api = Arc::new(DummyApi::start(ProviderFeatures::default()).await);
    let api_client = goose_providers::api_client::ApiClient::new_with_tls(
        api.uri(),
        goose_providers::api_client::AuthMethod::NoAuth,
        None,
    )?;
    let provider: Arc<dyn Provider> = Arc::new(
        goose_providers::openai::OpenAiProviderBuilder::new(api_client)
            .name(provider_name)
            .build(),
    );

    let temp_dir = tempfile::tempdir()?;
    let session_manager = Arc::new(SessionManager::new(temp_dir.path().to_path_buf()));
    let session = session_manager
        .create_session(
            temp_dir.path().to_path_buf(),
            "state-machine-reply".to_string(),
            SessionType::Hidden,
            GooseMode::Auto,
        )
        .await?;
    let agent = Agent::with_config(AgentConfig::new(
        session_manager,
        PermissionManager::instance(),
        None,
        GooseMode::Auto,
        true,
        GoosePlatform::GooseCli,
    ));
    agent
        .update_provider(
            provider,
            ModelConfig::new(goose_providers::openai::OPEN_AI_DEFAULT_MODEL)
                .with_canonical_limits("openai"),
            &session.id,
        )
        .await?;

    Ok((agent, api, session.id, temp_dir))
}

#[tokio::test]
async fn foreground_subagent_spike_runs_outside_delegate() -> Result<()> {
    let _guard = env_lock::lock_env([("GOOSE_STATE_MACHINE", Some("1"))]);
    let (agent, api, session_id, _temp_dir) =
        agent_with_named_dummy_api("subagent-spike-test").await?;
    agent
        .add_extension(
            ExtensionConfig::Platform {
                name: "summon".to_string(),
                description: "Delegate work".to_string(),
                display_name: None,
                bundled: None,
                available_tools: Vec::new(),
            },
            &session_id,
        )
        .await?;

    api.on("Delegate a child").call(
        "delegate",
        serde_json::json!({
            "instructions": "Return the number 42"
        }),
    );
    api.on("Return the number 42").call(
        FINAL_OUTPUT_TOOL_NAME,
        serde_json::json!({ "result": "42" }),
    );
    api.on("[subagent-result:")
        .reply("The foreground child returned 42.");

    let messages = reply_messages(
        &agent,
        session_id.clone(),
        Message::user().with_text("Delegate a child"),
    )
    .await?;

    let message_texts = messages
        .iter()
        .map(Message::as_concat_text)
        .collect::<Vec<_>>();
    let joined_messages = message_texts.join("");
    assert!(
        joined_messages.contains("The foreground child returned 42."),
        "expected parent continuation, got messages {message_texts:?} and {} API calls",
        api.call_count()
    );
    let calls = api.calls();
    let children = agent
        .config
        .session_manager
        .list_sessions_by_types(&[SessionType::SubAgent])
        .await?;
    let child = children
        .into_iter()
        .find(|child| child.parent_session_id.as_deref() == Some(session_id.as_str()))
        .expect("foreground delegate should persist a child session");
    let child = agent
        .config
        .session_manager
        .get_session(&child.id, true)
        .await?;
    assert!(
        calls
            .iter()
            .any(|call| call.input_contains("Delegate a child")),
        "parent delegation prompt did not reach the provider"
    );
    assert!(
        calls
            .iter()
            .any(|call| call.system_contains("specialized subagent")),
        "a reconstructed subagent runtime did not reach the provider"
    );
    assert!(
        calls
            .iter()
            .any(|call| call.input_contains("[subagent-result:")),
        "parent did not infer after child result delivery"
    );

    assert!(child
        .conversation
        .expect("child should have a persisted conversation")
        .messages()
        .iter()
        .any(|message| message.as_concat_text().contains("42")));

    Ok(())
}

#[tokio::test]
async fn foreground_subagent_spike_supports_nested_delegation() -> Result<()> {
    let _guard = env_lock::lock_env([("GOOSE_STATE_MACHINE", Some("1"))]);
    let (agent, api, session_id, _temp_dir) =
        agent_with_named_dummy_api("nested-subagent-spike-test").await?;
    agent
        .add_extension(
            ExtensionConfig::Platform {
                name: "summon".to_string(),
                description: "Delegate work".to_string(),
                display_name: None,
                bundled: None,
                available_tools: Vec::new(),
            },
            &session_id,
        )
        .await?;

    api.on("ROOT_DELEGATE_REQUEST").call(
        "delegate",
        serde_json::json!({ "instructions": "CHILD_MUST_DELEGATE" }),
    );
    api.on("CHILD_MUST_DELEGATE").call(
        "delegate",
        serde_json::json!({ "instructions": "GRANDCHILD_RETURN_42" }),
    );
    api.on("GRANDCHILD_RETURN_42").call(
        FINAL_OUTPUT_TOOL_NAME,
        serde_json::json!({ "result": "NESTED-42" }),
    );
    api.on("\"result\":\"NESTED-42\"").call(
        FINAL_OUTPUT_TOOL_NAME,
        serde_json::json!({ "result": "LEVEL_ONE_DONE" }),
    );
    api.on("\"result\":\"LEVEL_ONE_DONE\"").reply("ROOT_DONE");

    let messages = reply_messages(
        &agent,
        session_id.clone(),
        Message::user().with_text("ROOT_DELEGATE_REQUEST"),
    )
    .await?;
    assert!(messages
        .iter()
        .map(Message::as_concat_text)
        .collect::<String>()
        .contains("ROOT_DONE"));

    let calls = api.calls();
    assert_eq!(calls.len(), 5);
    assert_eq!(
        calls
            .iter()
            .filter(|call| call.system_contains("specialized subagent"))
            .count(),
        3
    );

    let children = agent
        .config
        .session_manager
        .list_sessions_by_types(&[SessionType::SubAgent])
        .await?;
    let child = children
        .iter()
        .find(|child| child.parent_session_id.as_deref() == Some(session_id.as_str()))
        .expect("parent should have a direct child");
    let grandchild = children
        .iter()
        .find(|candidate| candidate.parent_session_id.as_deref() == Some(child.id.as_str()))
        .expect("child should have its own child");
    let grandchild = agent
        .config
        .session_manager
        .get_session(&grandchild.id, true)
        .await?;
    let grandchild_conversation = grandchild
        .conversation
        .expect("grandchild should have a persisted conversation");
    assert_eq!(
        super::super::ops_recipe::RecipeOperation::successful_final_output(
            grandchild_conversation.messages()
        )
        .as_deref(),
        Some(r#"{"result":"NESTED-42"}"#)
    );

    Ok(())
}

#[tokio::test]
async fn background_subagent_spike_outlives_the_parent_turn() -> Result<()> {
    let _guard = env_lock::lock_env([("GOOSE_STATE_MACHINE", Some("1"))]);
    let (agent, api, session_id, _temp_dir) =
        agent_with_named_dummy_api("background-subagent-spike-test").await?;
    agent
        .add_extension(
            ExtensionConfig::Platform {
                name: "summon".to_string(),
                description: "Delegate work".to_string(),
                display_name: None,
                bundled: None,
                available_tools: Vec::new(),
            },
            &session_id,
        )
        .await?;

    api.on("START_BACKGROUND_CHILD").call(
        "delegate",
        serde_json::json!({
            "instructions": "BACKGROUND_CHILD_TASK",
            "async": true
        }),
    );
    let child_blocked = api
        .on_system("specialized subagent")
        .hold_reply("BACKGROUND_WORKING");
    api.on("BACKGROUND_WORKING").call(
        FINAL_OUTPUT_TOOL_NAME,
        serde_json::json!({ "result": "BACKGROUND_DONE" }),
    );
    api.on("Background subagent created")
        .reply("PARENT_CONTINUED");

    let messages = reply_messages(
        &agent,
        session_id.clone(),
        Message::user().with_text("START_BACKGROUND_CHILD"),
    )
    .await?;
    assert!(messages
        .iter()
        .map(Message::as_concat_text)
        .collect::<String>()
        .contains("PARENT_CONTINUED"));
    tokio::time::timeout(Duration::from_secs(5), child_blocked.entered()).await?;

    let children = agent
        .config
        .session_manager
        .list_sessions_by_types(&[SessionType::SubAgent])
        .await?;
    let child_id = children
        .into_iter()
        .find(|child| child.parent_session_id.as_deref() == Some(session_id.as_str()))
        .expect("background delegate should persist a child")
        .id;

    child_blocked.release();
    let completed = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let child = agent
                .config
                .session_manager
                .get_session(&child_id, true)
                .await?;
            if let Some(conversation) = child.conversation {
                if let Some(output) =
                    super::super::ops_recipe::RecipeOperation::successful_final_output(
                        conversation.messages(),
                    )
                {
                    return Ok::<_, anyhow::Error>(output);
                }
            }
            tokio::task::yield_now().await;
        }
    })
    .await??;
    assert_eq!(completed, r#"{"result":"BACKGROUND_DONE"}"#);

    Ok(())
}

#[tokio::test]
async fn reply_streams_the_turn_and_ends() -> Result<()> {
    let (agent, api, session_id, _temp_dir) = agent_with_dummy_api().await?;
    api.on("are you there?").reply("still here");

    let session_config = SessionConfig {
        id: session_id.clone(),
        schedule_id: None,
        max_turns: Some(2),
        retry_config: None,
    };
    let stream = agent
        .reply_with_state_machine(
            Message::user().with_text("are you there?"),
            session_config,
            Some(CancellationToken::new()),
        )
        .await?;

    let replies = tokio::time::timeout(Duration::from_secs(30), async move {
        tokio::pin!(stream);
        let mut replies = Vec::new();
        while let Some(event) = stream.next().await {
            if let AgentEvent::Message(message) = event? {
                replies.push(message.as_concat_text());
            }
        }
        anyhow::Ok(replies)
    })
    .await??;

    assert!(
        replies.iter().any(|reply| reply == "still here"),
        "expected the scripted reply, got {replies:?}"
    );
    assert_eq!(api.call_count(), 1);

    Ok(())
}

#[tokio::test]
async fn bang_shell_uses_the_state_machine_when_the_flag_is_disabled() -> Result<()> {
    let _guard = env_lock::lock_env([("GOOSE_STATE_MACHINE", None::<&str>)]);
    let (agent, api, session_id, _temp_dir) = agent_with_dummy_api().await?;
    let session_config = SessionConfig {
        id: session_id,
        schedule_id: None,
        max_turns: Some(2),
        retry_config: None,
    };
    let stream = agent
        .reply(
            Message::user().with_text("!echo hello"),
            session_config,
            Some(CancellationToken::new()),
        )
        .await?;
    tokio::pin!(stream);
    let mut requested_shell = false;
    while let Some(event) = stream.next().await {
        if let AgentEvent::Message(message) = event? {
            requested_shell |= message.content.iter().any(|content| {
                matches!(
                    content,
                    crate::conversation::message::MessageContent::ToolRequest(request)
                        if request.tool_call.as_ref().is_ok_and(|call| call.name == "shell")
                )
            });
        }
    }

    assert!(requested_shell);
    assert_eq!(api.call_count(), 0);

    Ok(())
}

async fn reply_messages(
    agent: &Agent,
    session_id: String,
    message: Message,
) -> Result<Vec<Message>> {
    let stream = agent
        .reply(
            message,
            SessionConfig {
                id: session_id,
                schedule_id: None,
                max_turns: Some(2),
                retry_config: None,
            },
            Some(CancellationToken::new()),
        )
        .await?;
    tokio::pin!(stream);
    let mut messages = Vec::new();
    while let Some(event) = stream.next().await {
        if let AgentEvent::Message(message) = event? {
            messages.push(message);
        }
    }
    Ok(messages)
}

fn assistant_only_acp_annotations() -> AcpAnnotations {
    AcpAnnotations::new().audience(vec![AcpRole::Assistant])
}

fn assistant_only_acp_text(text: &str) -> AcpContentBlock {
    AcpContentBlock::Text(AcpTextContent::new(text).annotations(assistant_only_acp_annotations()))
}

fn empty_audience_acp_annotations() -> AcpAnnotations {
    AcpAnnotations::new().audience(Vec::new())
}

fn empty_audience_acp_text(text: &str) -> AcpContentBlock {
    AcpContentBlock::Text(AcpTextContent::new(text).annotations(empty_audience_acp_annotations()))
}

fn assistant_only_embedded_resource(text: &str) -> AcpContentBlock {
    AcpContentBlock::Resource(
        EmbeddedResource::new(EmbeddedResourceResource::TextResourceContents(
            TextResourceContents::new(text, "file:///hidden-resource.txt"),
        ))
        .annotations(assistant_only_acp_annotations()),
    )
}

fn empty_audience_embedded_resource(text: &str) -> AcpContentBlock {
    AcpContentBlock::Resource(
        EmbeddedResource::new(EmbeddedResourceResource::TextResourceContents(
            TextResourceContents::new(text, "file:///empty-audience-resource.txt"),
        ))
        .annotations(empty_audience_acp_annotations()),
    )
}

fn assistant_only_resource_link(text: &str) -> Result<(AcpContentBlock, tempfile::NamedTempFile)> {
    let file = tempfile::NamedTempFile::new()?;
    std::fs::write(file.path(), text)?;
    let uri = url::Url::from_file_path(file.path())
        .map_err(|()| anyhow::anyhow!("temporary resource path is not a valid file URL"))?;
    let link = ResourceLink::new("hidden-resource.txt", uri.to_string())
        .annotations(assistant_only_acp_annotations());
    Ok((AcpContentBlock::ResourceLink(link), file))
}

fn shell_commands(messages: &[Message]) -> Vec<&str> {
    messages
        .iter()
        .flat_map(|message| &message.content)
        .filter_map(|content| match content {
            MessageContent::ToolRequest(request) => request
                .tool_call
                .as_ref()
                .ok()
                .filter(|call| call.name == "shell")
                .and_then(|call| call.arguments.as_ref())
                .and_then(|arguments| arguments.get("command"))
                .and_then(serde_json::Value::as_str),
            _ => None,
        })
        .collect()
}

async fn assert_bang_shell_uses_only_user_visible_content() -> Result<()> {
    let (agent, api, session_id, _temp_dir) = agent_with_dummy_api().await?;
    api.on("benign visible input")
        .reply("handled as ordinary input");
    let hidden_text_prefix = GooseAcpAgent::convert_acp_prompt_to_message(&[
        assistant_only_acp_text("!echo hidden"),
        AcpContentBlock::Text(AcpTextContent::new("benign visible input")),
    ]);
    let messages = reply_messages(&agent, session_id, hidden_text_prefix).await?;
    assert!(shell_commands(&messages).is_empty());
    assert_eq!(api.call_count(), 1);

    let (agent, api, session_id, _temp_dir) = agent_with_dummy_api().await?;
    api.on("benign visible input")
        .reply("handled as ordinary input");
    let empty_audience_text = GooseAcpAgent::convert_acp_prompt_to_message(&[
        empty_audience_acp_text("!echo hidden"),
        AcpContentBlock::Text(AcpTextContent::new("benign visible input")),
    ]);
    let messages = reply_messages(&agent, session_id, empty_audience_text).await?;
    assert!(shell_commands(&messages).is_empty());
    assert_eq!(api.call_count(), 1);

    let (agent, api, session_id, _temp_dir) = agent_with_dummy_api().await?;
    let hidden_text_suffix = GooseAcpAgent::convert_acp_prompt_to_message(&[
        AcpContentBlock::Text(AcpTextContent::new("!echo visible")),
        assistant_only_acp_text("&& echo hidden"),
    ]);
    let messages = reply_messages(&agent, session_id, hidden_text_suffix).await?;
    assert_eq!(shell_commands(&messages), ["echo visible"]);
    assert_eq!(api.call_count(), 0);

    let (agent, api, session_id, _temp_dir) = agent_with_dummy_api().await?;
    api.on("benign visible input")
        .reply("handled as ordinary input");
    let hidden_resource_prefix = GooseAcpAgent::convert_acp_prompt_to_message(&[
        assistant_only_embedded_resource("!echo hidden"),
        AcpContentBlock::Text(AcpTextContent::new("benign visible input")),
    ]);
    let messages = reply_messages(&agent, session_id, hidden_resource_prefix).await?;
    assert!(shell_commands(&messages).is_empty());
    assert_eq!(api.call_count(), 1);

    let (agent, api, session_id, _temp_dir) = agent_with_dummy_api().await?;
    api.on("benign visible input")
        .reply("handled as ordinary input");
    let empty_audience_resource = GooseAcpAgent::convert_acp_prompt_to_message(&[
        empty_audience_embedded_resource("!echo hidden"),
        AcpContentBlock::Text(AcpTextContent::new("benign visible input")),
    ]);
    let messages = reply_messages(&agent, session_id, empty_audience_resource).await?;
    assert!(shell_commands(&messages).is_empty());
    assert_eq!(api.call_count(), 1);

    let (agent, api, session_id, _temp_dir) = agent_with_dummy_api().await?;
    let (hidden_link, _resource_file) = assistant_only_resource_link("&& echo hidden")?;
    let hidden_link_suffix = GooseAcpAgent::convert_acp_prompt_to_message(&[
        AcpContentBlock::Text(AcpTextContent::new("!echo visible")),
        hidden_link,
    ]);
    let messages = reply_messages(&agent, session_id, hidden_link_suffix).await?;
    assert_eq!(shell_commands(&messages), ["echo visible"]);
    assert_eq!(api.call_count(), 0);

    Ok(())
}

#[tokio::test]
async fn bang_shell_visibility_is_enforced_when_state_machine_is_disabled() -> Result<()> {
    let _guard = env_lock::lock_env([("GOOSE_STATE_MACHINE", None::<&str>)]);
    assert_bang_shell_uses_only_user_visible_content().await
}

#[tokio::test]
async fn bang_shell_visibility_is_enforced_when_state_machine_is_enabled() -> Result<()> {
    let _guard = env_lock::lock_env([("GOOSE_STATE_MACHINE", Some("1"))]);
    assert_bang_shell_uses_only_user_visible_content().await
}
