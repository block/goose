use anyhow::Result;

/// Run the ACP agent server
pub async fn run_acp_agent() -> Result<()> {
    goose::acp::run_acp_agent().await
}
