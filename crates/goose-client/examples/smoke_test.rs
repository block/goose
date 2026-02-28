//! Smoke test: verify goose-client against a running goose-server.
//!
//! Start goosed first:
//!   GOOSE_SERVER__SECRET_KEY=test cargo run -p goose-server --bin goosed -- agent
//!
//! Then run:
//!   cargo run -p goose-client --example smoke_test

use goose_client::{GooseClient, GooseClientConfig, StartAgentRequest};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();

    let url =
        std::env::var("GOOSE_SERVER_URL").unwrap_or_else(|_| "https://127.0.0.1:3000".to_string());
    let secret = std::env::var("GOOSE_SERVER_SECRET_KEY").unwrap_or_else(|_| "test".to_string());

    let client = GooseClient::new(GooseClientConfig::new(&url, &secret))?;
    println!("Connecting to {url}");

    let status = client.status().await?;
    assert_eq!(status, "ok");
    println!("[ok] status");

    let dir = std::env::temp_dir().join("goose-client-smoke-test");
    std::fs::create_dir_all(&dir)?;
    let session = client
        .start_agent(StartAgentRequest::new(dir.to_string_lossy()))
        .await?;
    println!("[ok] start_agent -> session {}", session.id);

    let list = client.list_sessions().await?;
    println!("[ok] list_sessions ({} total)", list.sessions.len());

    let fetched = client.get_session(&session.id).await?;
    assert_eq!(fetched.id, session.id);
    println!("[ok] get_session");

    client.rename_session(&session.id, "smoke-test").await?;
    println!("[ok] rename_session");

    let insights = client.get_session_insights().await?;
    println!(
        "[ok] get_session_insights -> {} sessions, {} tokens",
        insights.total_sessions, insights.total_tokens
    );

    let info = client.system_info().await?;
    println!("[ok] system_info -> {} {}", info.os, info.os_version);

    client.stop_agent(&session.id).await?;
    println!("[ok] stop_agent");

    client.delete_session(&session.id).await?;
    println!("[ok] delete_session");

    println!("\nAll smoke tests passed.");
    Ok(())
}
