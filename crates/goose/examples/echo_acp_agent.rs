//! Minimal ACP agent that echoes back prompt text via session notifications.
//! Used for E2E testing of AgentClientManager.

use agent_client_protocol::{Agent, AgentSideConnection, Client};
use agent_client_protocol_schema::{
    AuthenticateRequest, AuthenticateResponse, CancelNotification, ContentBlock, ContentChunk,
    InitializeRequest, InitializeResponse, NewSessionRequest, NewSessionResponse, PromptRequest,
    PromptResponse, ProtocolVersion, SessionId, SessionNotification, SessionUpdate,
    SetSessionModeRequest, SetSessionModeResponse, StopReason, TextContent,
};
use async_trait::async_trait;
use std::cell::RefCell;
use std::rc::Rc;
use tokio::io::{stdin, stdout};
use tokio::task::LocalSet;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

struct EchoAgent {
    conn: RefCell<Option<Rc<AgentSideConnection>>>,
}

impl EchoAgent {
    fn new() -> Self {
        Self {
            conn: RefCell::new(None),
        }
    }

    fn set_conn(&self, conn: Rc<AgentSideConnection>) {
        *self.conn.borrow_mut() = Some(conn);
    }
}

#[async_trait(?Send)]
impl Agent for EchoAgent {
    async fn initialize(
        &self,
        _req: InitializeRequest,
    ) -> Result<InitializeResponse, agent_client_protocol_schema::Error> {
        Ok(InitializeResponse::new(ProtocolVersion::LATEST))
    }

    async fn authenticate(
        &self,
        _req: AuthenticateRequest,
    ) -> Result<AuthenticateResponse, agent_client_protocol_schema::Error> {
        Ok(AuthenticateResponse::new())
    }

    async fn new_session(
        &self,
        _req: NewSessionRequest,
    ) -> Result<NewSessionResponse, agent_client_protocol_schema::Error> {
        Ok(NewSessionResponse::new(SessionId::from(
            "echo-session-1".to_string(),
        )))
    }

    async fn prompt(
        &self,
        req: PromptRequest,
    ) -> Result<PromptResponse, agent_client_protocol_schema::Error> {
        let texts: Vec<String> = req
            .prompt
            .iter()
            .filter_map(|block| match block {
                ContentBlock::Text(t) => Some(t.text.clone()),
                _ => None,
            })
            .collect();

        let echo_text = if texts.is_empty() {
            "echo: <no text>".to_string()
        } else {
            format!("echo: {}", texts.join(" "))
        };

        let conn = self.conn.borrow().clone();
        if let Some(conn) = conn.as_ref() {
            let chunk = ContentChunk::new(ContentBlock::Text(TextContent::new(echo_text)));
            let update = SessionUpdate::AgentMessageChunk(chunk);
            let notification =
                SessionNotification::new(SessionId::from("echo-session-1".to_string()), update);
            let _ = conn.session_notification(notification).await;
        }

        Ok(PromptResponse::new(StopReason::EndTurn))
    }

    async fn cancel(
        &self,
        _req: CancelNotification,
    ) -> Result<(), agent_client_protocol_schema::Error> {
        Ok(())
    }

    async fn set_session_mode(
        &self,
        _req: SetSessionModeRequest,
    ) -> Result<SetSessionModeResponse, agent_client_protocol_schema::Error> {
        Ok(SetSessionModeResponse::new())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let local_set = LocalSet::new();
    local_set
        .run_until(async {
            let agent = Rc::new(EchoAgent::new());
            let stdin = stdin().compat();
            let stdout = stdout().compat_write();

            let (conn, io_task) = AgentSideConnection::new(agent.clone(), stdout, stdin, |fut| {
                tokio::task::spawn_local(fut);
            });

            let conn = Rc::new(conn);
            agent.set_conn(conn);

            io_task
                .await
                .map_err(|e| anyhow::anyhow!("IO task failed: {e}"))?;
            Ok(())
        })
        .await
}
