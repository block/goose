use anyhow::Result;
use goose_cli::cli::cli;

#[tokio::main]
async fn main() -> Result<()> {
    cli().await
}
