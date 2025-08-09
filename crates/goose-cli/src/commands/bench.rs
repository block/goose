use crate::session::build_session;
use crate::session::SessionBuilderConfig;
use crate::{logging, session, Session};
use async_trait::async_trait;
use goose::conversation::Conversation;
use goose_bench::bench_session::{BenchAgent, BenchBaseSession};
use goose_bench::eval_suites::ExtensionRequirements;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

// allow session obj to be used in benchmarking
#[async_trait]
impl BenchBaseSession for Session {
    async fn headless(&mut self, message: String) -> anyhow::Result<()> {
        self.headless(message).await
    }
    fn session_file(&self) -> Option<PathBuf> {
        self.session_file()
    }
    fn message_history(&self) -> Conversation {
        self.message_history()
    }
    fn get_total_token_usage(&self) -> anyhow::Result<Option<i32>> {
        self.get_total_token_usage()
    }
}
pub async fn agent_generator(
    requirements: ExtensionRequirements,
    session_id: String,
) -> BenchAgent {
    let identifier = Some(session::Identifier::Name(session_id));

    let base_session = build_session(SessionBuilderConfig {
        identifier,
        resume: false,
        no_session: false,
        extensions: requirements.external,
        remote_extensions: requirements.remote,
        streamable_http_extensions: Vec::new(),
        builtins: requirements.builtin,
        extensions_override: None,
        additional_system_prompt: None,
        settings: None,
        provider: None,
        model: None,
        debug: false,
        max_tool_repetitions: None,
        interactive: false, // Benchmarking is non-interactive
        scheduled_job_id: None,
        max_turns: None,
        quiet: false,
        sub_recipes: None,
        final_output_response: None,
        retry_config: None,
    })
    .await;

    // package session obj into benchmark-compatible struct
    let bench_agent = BenchAgent::new(Box::new(base_session));

    // Initialize logging with error capture
    let errors = Some(Arc::new(Mutex::new(bench_agent.get_errors().await)));
    logging::setup_logging(Some("bench"), errors).expect("Failed to initialize logging");

    bench_agent
}
