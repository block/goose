//! goose-demo - A minimal AI agent
//!
//! Run with:
//!   GOOSE_PROVIDER=openai GOOSE_MODEL=gpt-4o OPENAI_API_KEY=... cargo run

use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing to stderr (stdout is for ACP protocol)
    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(std::io::stderr))
        .with(EnvFilter::from_default_env().add_directive("goose_demo=info".parse()?))
        .init();

    // Run the server
    let server = goose_demo::Server::new()?;
    server.run().await?;

    Ok(())
}
