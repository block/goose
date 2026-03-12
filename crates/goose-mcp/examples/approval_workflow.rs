use anyhow::Result;
use goose_mcp::mcp_server_runner::serve;
use goose_mcp::ApprovalServer;

#[tokio::main]
async fn main() -> Result<()> {
    serve(ApprovalServer::new()).await
}
