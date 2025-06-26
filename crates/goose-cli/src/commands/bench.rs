// Logging is handled by CLI entry point
use crate::session::{build_session, Session, SessionBuilderConfig};
use async_trait::async_trait;
use goose::message::Message;
use goose::session::Identifier;
use goose_bench::bench_session::{BenchAgent, BenchBaseSession};
use goose_bench::eval_suites::ExtensionRequirements;
use mcp_core::Tool;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;


struct SessionWrapper {
    session: Session,
    _dummy_messages: Vec<Message>,
}

impl SessionWrapper {
    fn new(session: Session) -> Self {
        Self {
            session,
            _dummy_messages: Vec::new(),
        }
    }
}

// allow session obj to be used in benchmarking
#[async_trait]
impl BenchBaseSession for SessionWrapper {
    async fn headless(&mut self, message: String) -> anyhow::Result<()> {
        self.session.headless(message).await
    }
    fn session_file(&self) -> PathBuf {
        self.session.session_file()
    }
    fn message_history(&self) -> Vec<Message> {
        self.session.message_history()
    }
    async fn override_system_prompt(&self, override_prompt: String) {
        self.session.override_system_prompt(override_prompt).await
    }
    fn get_total_token_usage(&self) -> anyhow::Result<Option<i32>> {
        self.session.get_total_token_usage()
    }
    async fn cleanup_extensions(&self) -> anyhow::Result<()> {
        self.session.cleanup_extensions().await
    }
    fn get_agent(&self) -> &goose::agents::Agent {
        self.session.get_agent()
    }
    fn get_messages_mut(&mut self) -> &mut Vec<Message> {
        &mut self._dummy_messages
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub async fn agent_generator(
    requirements: ExtensionRequirements,
    session_id: String,
    dataset: bool,
    available_extensions: Option<HashMap<String, Vec<Tool>>>,
) -> BenchAgent {
    if dataset {
        // Delegate dataset agent creation to goose-bench's implementation
        goose_bench::agent_generator::agent_generator(requirements, session_id, dataset, available_extensions).await
    } else {
        standard_agent(requirements, session_id, available_extensions).await
    }
}

async fn standard_agent(
    requirements: ExtensionRequirements,
    session_id: String,
    _: Option<HashMap<String, Vec<Tool>>>,
) -> BenchAgent {
    let identifier = Some(Identifier::Name(session_id));

    let base_session = build_session(SessionBuilderConfig {
        identifier,
        resume: false,
        no_session: false,
        extensions: requirements.external,
        remote_extensions: requirements.remote,
        builtins: requirements.builtin,
        extensions_override: None,
        additional_system_prompt: None,
        settings: None,
        debug: false,
        max_tool_repetitions: None,
        interactive: false, // Benchmarking is non-interactive
        scheduled_job_id: None,
        quiet: false,
        sub_recipes: None,
    })
    .await;

    // package session obj into benchmark-compatible struct
    let bench_agent = BenchAgent::new(Box::new(SessionWrapper::new(base_session)));

    // Logging is already set up by CLI - just register error capture if needed
    let errors = Arc::new(Mutex::new(bench_agent.get_errors().await));
    goose_bench::error_capture::ErrorCaptureLayer::register_error_vector(errors);

    bench_agent
}
