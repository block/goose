use anyhow::Result;

use crate::session::SessionBuilderConfig;
use crate::session::build_session;

pub async fn handle_doctor() -> Result<()> {
    let mut session = build_session(SessionBuilderConfig {
        no_session: true,
        interactive: true,
        ..Default::default()
    })
    .await;

    session.interactive(Some("/doctor".to_string())).await
}
