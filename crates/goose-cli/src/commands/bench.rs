use crate::cli::StreamableHttpOptions;
use crate::session::build_session;
use crate::session::SessionBuilderConfig;
use crate::{logging, CliSession};
use async_trait::async_trait;
use goose::conversation::Conversation;
use goose::session::session_manager::Session;
use goose_bench::bench_session::{BenchAgent, BenchBaseSession};
use goose_bench::eval_suites::ExtensionRequirements;
use std::sync::Arc;
use tokio::sync::Mutex;

// allow session obj to be used in benchmarking
#[async_trait]
impl BenchBaseSession for CliSession {
    async fn headless(&mut self, message: String) -> anyhow::Result<()> {
        self.headless(message).await
    }
    fn message_history(&self) -> Conversation {
        self.message_history()
    }
    fn get_total_token_usage(&self) -> anyhow::Result<Option<i32>> {
        // Since the trait requires sync but the session method is async,
        // we need to block on the async call
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(self.get_total_token_usage())
        })
    }

    async fn get_session(&self) -> anyhow::Result<Session> {
        self.get_session().await
    }
}
pub async fn agent_generator(
    requirements: ExtensionRequirements,
    session_id: String,
) -> BenchAgent {
    let streamable_http_extensions: Vec<StreamableHttpOptions> = requirements
        .streamable_http
        .iter()
        .map(|s| StreamableHttpOptions {
            url: s.clone(),
            timeout: goose::config::DEFAULT_EXTENSION_TIMEOUT,
        })
        .collect();
    let base_session = build_session(SessionBuilderConfig {
        session_id: Some(session_id),
        resume: false,
        fork: false,
        no_session: false,
        extensions: requirements.external,
        streamable_http_extensions,
        builtins: requirements.builtin,
        recipe: None,
        additional_system_prompt: None,
        provider: None,
        model: None,
        debug: false,
        max_tool_repetitions: None,
        interactive: false, // Benchmarking is non-interactive
        scheduled_job_id: None,
        max_turns: None,
        quiet: false,
        output_format: "text".to_string(),
        container: None,
    })
    .await;

    let bench_agent = BenchAgent::new(Box::new(base_session));

    let errors = Some(Arc::new(Mutex::new(bench_agent.get_errors().await)));
    logging::setup_logging(Some("bench"), errors).expect("Failed to initialize logging");

    bench_agent
}
