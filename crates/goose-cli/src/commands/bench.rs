use crate::session::build_session;
use crate::{logging, Session};
use async_trait::async_trait;
use goose::message::Message;
use goose_bench::bench_session::{BenchAgent, BenchBaseSession, BenchSession};
use goose_bench::eval_suites::ExtensionRequirements;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

#[async_trait]
impl BenchBaseSession for Session {
    async fn headless(&mut self, message: String) -> anyhow::Result<()> {
        self.headless(message).await
    }
    fn session_file(&self) -> PathBuf {
        self.session_file()
    }
    fn message_history(&self) -> Vec<Message> {
        self.message_history()
    }
    fn get_total_token_usage(&self) -> anyhow::Result<Option<i32>> {
        self.get_total_token_usage()
    }
}
pub async fn agent_generator(requirements: ExtensionRequirements) -> Box<dyn BenchAgent> {
    let base_session = build_session(
        None,
        false,
        requirements.external,
        requirements.builtin,
        false,
    )
    .await;

    let _run_id2 = base_session.session_file().file_stem();

    let bench_agent = BenchSession::new(Box::new(base_session));

    // Initialize logging with error capture
    let errors = Some(Arc::new(Mutex::new(bench_agent.get_errors().await)));
    logging::setup_logging(Some("bench"), errors).expect("Failed to initialize logging");

    // Create session with error capture
    Box::new(bench_agent)
}
