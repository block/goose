use anyhow::Result;
use gosling::providers::utils::init_gosling_request_log;
use tracing_subscriber::util::SubscriberInitExt;

/// Sets up the logging infrastructure for the server.
/// Logs go to a JSON file and a pretty console layer on stderr.
pub fn setup_logging(name: Option<&str>) -> Result<()> {
    init_gosling_request_log()?;
    let config = gosling::logging::LoggingConfig {
        component: "server",
        name,
        extra_directives: &["gosling_server=info", "tower_http=info"],
        console: true,
        json: false,
    };
    let subscriber = gosling::logging::build_logging_subscriber(&config)?;
    subscriber.try_init()?;
    Ok(())
}
