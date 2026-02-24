use std::sync::Arc;

use anyhow::Result;
use base64::{engine::general_purpose, Engine};
use goose::agents::Agent;
use goose::config::ExtensionConfig;
use goose::providers::fixture_provider::FixtureProvider;
use goose::recipe::Recipe;
use goose_cli::scenario_tests::mock_client;
use goose_server::test_support::spawn_test_server;
use serial_test::serial;
use tempfile::TempDir;
use tokio::sync::Mutex;
use tokio_stream::StreamExt;

fn recipe_with_no_extensions() -> Recipe {
    Recipe {
        version: "1.0.0".to_string(),
        title: "Scenario Test".to_string(),
        description: "Hermetic scenario test".to_string(),
        instructions: Some("You are goose.".to_string()),
        prompt: None,
        extensions: Some(vec![]),
        settings: None,
        activities: None,
        author: None,
        parameters: None,
        response: None,
        sub_recipes: None,
        retry: None,
    }
}

fn image_message(prompt: &str) -> goose::conversation::message::Message {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let image_path = format!(
        "{}/src/scenario_tests/test_data/test_image.jpg",
        manifest_dir
    );

    let image_data = std::fs::read(image_path).expect("Failed to read image");
    let base64_data = general_purpose::STANDARD.encode(&image_data);

    goose::conversation::message::Message::user()
        .with_text(prompt)
        .with_image(base64_data, "image/jpeg")
}

fn fixture_path(test_name: &str) -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!(
        "{}/src/scenario_tests/fixtures/{}.json",
        manifest_dir, test_name
    )
}

fn fixture_expected_text(test_name: &str) -> Result<String> {
    let data: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(fixture_path(test_name))?)?;
    let steps = data
        .get("steps")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow::anyhow!("fixture missing steps"))?;

    // last text step wins
    for step in steps.iter().rev() {
        let output = step.get("output").unwrap_or(step);
        if output.get("type").and_then(|t| t.as_str()) == Some("text") {
            if let Some(text) = output.get("text").and_then(|t| t.as_str()) {
                return Ok(text.to_string());
            }
        }
    }

    Ok(String::new())
}

fn write_global_hints(root: &std::path::Path) {
    let config_dir = root.join("config");
    std::fs::create_dir_all(&config_dir).expect("create config dir");

    std::fs::write(
        config_dir.join(".goosehints"),
        "These are my global goose hints.\nThese are my global goose hints.\n",
    )
    .expect("write .goosehints");
}

async fn setup(
    test_name: &str,
) -> Result<(
    goose_server::test_support::TestServerHandle,
    goose_cli::GoosedClient,
    String,
    TempDir,
)> {
    let path_root = TempDir::new()?;
    std::env::set_var("GOOSE_PATH_ROOT", path_root.path());
    write_global_hints(path_root.path());

    let server = spawn_test_server().await?;
    let goosed = goose_cli::GoosedClient::connect(&server.base_url, &server.secret_key)?;

    let working_dir = TempDir::new()?;
    let session = goosed
        .start_agent(
            working_dir.path().to_str().expect("utf8 working dir"),
            Some(&recipe_with_no_extensions()),
            Some(vec![]),
        )
        .await?;

    let agent: Arc<Agent> = server.state.get_agent(session.id.clone()).await?;

    // Deterministic, ordered fixture provider for this session.
    let provider = FixtureProvider::from_file(fixture_path(test_name))?;
    agent
        .update_provider(Arc::new(provider), &session.id)
        .await?;

    // Hermetic weather extension for tool scenarios.
    agent
        .extension_manager
        .add_client(
            "weather_extension".to_string(),
            ExtensionConfig::Builtin {
                name: "".to_string(),
                display_name: None,
                description: "".to_string(),
                timeout: None,
                bundled: None,
                available_tools: vec![],
            },
            Arc::new(Mutex::new(Box::new(mock_client::weather_client()))),
            None,
            None,
        )
        .await;

    Ok((server, goosed, session.id, path_root))
}

fn last_assistant_text(conversation: &goose::conversation::Conversation) -> String {
    conversation
        .messages()
        .iter()
        .rev()
        .find_map(|m| {
            if m.role == rmcp::model::Role::Assistant {
                let text = m.as_concat_text();
                (!text.trim().is_empty()).then_some(text)
            } else {
                None
            }
        })
        .unwrap_or_default()
}

async fn run_one_message(
    goosed: &goose_cli::GoosedClient,
    session_id: &str,
    message: goose::conversation::message::Message,
) -> Result<goose::conversation::Conversation> {
    let handle = goosed.handle();
    let mut stream =
        tokio_stream::wrappers::ReceiverStream::new(handle.reply(session_id, message, None).await?);

    let mut last_sse_assistant_text: Option<String> = None;

    while let Some(event) = stream.next().await {
        let event = event?;

        if std::env::var_os("GOOSE_SCENARIO_DEBUG").is_some() {
            eprintln!("SSE: {event:?}");
        }

        match event {
            goose_cli::goosed_client::types::SseEvent::Message { message, .. } => {
                if message.role == rmcp::model::Role::Assistant {
                    let text = message.as_concat_text();
                    if !text.trim().is_empty() {
                        last_sse_assistant_text = Some(text);
                    }
                }
            }
            goose_cli::goosed_client::types::SseEvent::Finish { .. } => break,
            goose_cli::goosed_client::types::SseEvent::Error { error } => {
                return Err(anyhow::anyhow!("server error: {}", error));
            }
            _ => {}
        }
    }

    let session = goosed.get_session(session_id).await?;
    if std::env::var_os("GOOSE_SCENARIO_DEBUG").is_some() {
        let conv_len = session.conversation.as_ref().map(|c| c.len()).unwrap_or(0);
        eprintln!("Session conversation length: {conv_len}");
    }

    let mut conversation = session.conversation.unwrap_or_default();
    if conversation.is_empty() {
        if let Some(text) = last_sse_assistant_text {
            conversation.push(goose::conversation::message::Message::assistant().with_text(text));
        }
    }

    Ok(conversation)
}

#[tokio::test]
#[serial]
async fn test_what_is_your_name() -> Result<()> {
    let (_server, goosed, session_id, _path_root) = setup("what_is_your_name").await?;

    let conversation = run_one_message(
        &goosed,
        &session_id,
        goose::conversation::message::Message::user().with_text("what is your name"),
    )
    .await?;

    let actual = last_assistant_text(&conversation);
    let expected = fixture_expected_text("what_is_your_name")?;
    assert_eq!(actual.trim(), expected.trim());

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_weather_tool() -> Result<()> {
    let (_server, goosed, session_id, _path_root) = setup("weather_tool").await?;

    let conversation = run_one_message(
        &goosed,
        &session_id,
        goose::conversation::message::Message::user()
            .with_text("tell me what the weather is in Berlin, Germany"),
    )
    .await?;

    let actual = last_assistant_text(&conversation);
    let expected = fixture_expected_text("weather_tool")?;
    assert_eq!(actual.trim(), expected.trim());

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_image_analysis() -> Result<()> {
    let (_server, goosed, session_id, _path_root) = setup("image_analysis").await?;

    let conversation = run_one_message(
        &goosed,
        &session_id,
        image_message("What do you see in this image?"),
    )
    .await?;

    let actual = last_assistant_text(&conversation);
    let expected = fixture_expected_text("image_analysis")?;
    assert_eq!(actual.trim(), expected.trim());

    Ok(())
}
