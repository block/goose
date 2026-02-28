//! Streaming chat: verify SSE streaming against a running goose-server.
//!
//! Requires a configured LLM provider. Start goosed first:
//!   GOOSE_SERVER__SECRET_KEY=test cargo run -p goose-server --bin goosed -- agent
//!
//! Then run:
//!   cargo run -p goose-client --example chat_stream

use futures::StreamExt;
use goose_client::{GooseClient, GooseClientConfig, MessageEvent, StartAgentRequest};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();

    let url =
        std::env::var("GOOSE_SERVER_URL").unwrap_or_else(|_| "https://127.0.0.1:3000".to_string());
    let secret = std::env::var("GOOSE_SERVER_SECRET_KEY").unwrap_or_else(|_| "test".to_string());

    let client = GooseClient::new(GooseClientConfig::new(&url, &secret))?;
    println!("Connecting to {url}");

    let dir = std::env::temp_dir().join("goose-client-chat-test");
    std::fs::create_dir_all(&dir)?;
    let session = client
        .start_agent(StartAgentRequest::new(dir.to_string_lossy()))
        .await?;
    println!("Started session {}\n", session.id);

    let mut stream = client
        .send_message(&session.id, "Say exactly: hello world")
        .await?;

    let mut got_message = false;
    let mut got_finish = false;

    while let Some(event) = stream.next().await {
        match event? {
            MessageEvent::Message { message, .. } => {
                got_message = true;
                println!("[message] {:?}", message);
            }
            MessageEvent::Finish { reason, .. } => {
                got_finish = true;
                println!("[finish] reason={reason}");
            }
            MessageEvent::Ping => {}
            other => println!("[event] {:?}", other),
        }
    }

    assert!(got_message, "expected at least one Message event");
    assert!(got_finish, "expected a Finish event");

    client.stop_agent(&session.id).await?;
    client.delete_session(&session.id).await?;
    println!("\nStreaming test passed.");
    Ok(())
}
