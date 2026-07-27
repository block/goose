use crate::acp::server::{AcpProviderFactory, GooseAcpAgent, GooseAcpAgentOptions};
use crate::agents::GoosePlatform;
use crate::scheduler_trait::SchedulerTrait;
use crate::session::SessionManager;
use crate::source_roots::SourceRoot;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::OnceCell;
use tracing::info;

/// Environment variable that suppresses the cron scheduler in `goose acp`.
///
/// Hosts that run a pool of ACP workers (for example Buzz, which spawns one
/// `goose acp` child per agent slot) would otherwise have every worker execute
/// the user's personal schedule, so a single cron entry fires once per worker.
/// Such hosts set this variable on every child they spawn.
pub const GOOSE_ACP_SCHEDULER_DISABLED_ENV: &str = "GOOSE_ACP_SCHEDULER_DISABLED";

/// Reads [`GOOSE_ACP_SCHEDULER_DISABLED`] straight from the process
/// environment.
///
/// Deliberately not routed through [`crate::config::Config`]: this must be a
/// read-only, environment-only switch, and the config layer both falls back to
/// persisted YAML and coerces values like `"1"` into JSON numbers that fail to
/// deserialize as `bool`.
///
/// Only `true` disables scheduling, matched case-insensitively after trimming
/// surrounding whitespace. Unset, `false`, and unparseable values all leave
/// scheduling enabled, so a typo can never silently disable a user's cron jobs.
///
/// [`GOOSE_ACP_SCHEDULER_DISABLED`]: GOOSE_ACP_SCHEDULER_DISABLED_ENV
fn scheduler_disabled_by_env() -> bool {
    std::env::var(GOOSE_ACP_SCHEDULER_DISABLED_ENV)
        .is_ok_and(|value| value.trim().eq_ignore_ascii_case("true"))
}

pub struct AcpServerFactoryConfig {
    pub builtins: Vec<String>,
    pub data_dir: std::path::PathBuf,
    pub config_dir: std::path::PathBuf,
    pub goose_platform: GoosePlatform,
    pub additional_source_roots: Vec<SourceRoot>,
}

pub struct AcpServer {
    config: AcpServerFactoryConfig,
    scheduler: OnceCell<Arc<dyn SchedulerTrait>>,
}

impl AcpServer {
    pub fn new(config: AcpServerFactoryConfig) -> Self {
        Self {
            config,
            scheduler: OnceCell::new(),
        }
    }

    /// Returns the scheduler for ACP agents, or `None` when scheduling is
    /// disabled for this runtime.
    ///
    /// When disabled the [`crate::scheduler::Scheduler`] is never constructed,
    /// so `schedule.json` is neither read nor written.
    async fn scheduler(&self) -> Result<Option<Arc<dyn SchedulerTrait>>> {
        if scheduler_disabled_by_env() {
            info!(
                "{GOOSE_ACP_SCHEDULER_DISABLED_ENV}=true; scheduled recipes are disabled in this ACP runtime"
            );
            return Ok(None);
        }

        let data_dir = self.config.data_dir.clone();
        self.scheduler
            .get_or_try_init(|| async move {
                let session_manager = Arc::new(SessionManager::new(data_dir.clone()));
                let schedule_file_path = data_dir.join("schedule.json");
                let scheduler =
                    crate::scheduler::Scheduler::new(schedule_file_path, session_manager)
                        .await
                        .map(|scheduler| scheduler as Arc<dyn SchedulerTrait>)?;
                Ok(scheduler)
            })
            .await
            .cloned()
            .map(Some)
    }

    pub async fn create_agent(&self) -> Result<Arc<GooseAcpAgent>> {
        let config = crate::config::Config::global();
        let disable_session_naming = config.get_goose_disable_session_naming().unwrap_or(false);
        let scheduler = self.scheduler().await?;

        let provider_factory: AcpProviderFactory =
            Arc::new(move |provider_name, extensions, working_dir| {
                Box::pin(async move {
                    match working_dir {
                        Some(working_dir) => {
                            crate::providers::create_with_working_dir(
                                &provider_name,
                                extensions,
                                working_dir,
                            )
                            .await
                        }
                        None => crate::providers::create(&provider_name, extensions).await,
                    }
                })
            });

        let agent = GooseAcpAgent::new(GooseAcpAgentOptions {
            provider_factory,
            builtins: self.config.builtins.clone(),
            data_dir: self.config.data_dir.clone(),
            config_dir: self.config.config_dir.clone(),
            disable_session_naming,
            goose_platform: self.config.goose_platform.clone(),
            additional_source_roots: self.config.additional_source_roots.clone(),
            scheduler,
        })
        .await?;
        info!("Created new ACP agent");

        Ok(Arc::new(agent))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use test_case::test_case;

    #[test_case(None, false ; "unset keeps scheduling enabled")]
    #[test_case(Some("true"), true ; "true disables scheduling")]
    #[test_case(Some("TRUE"), true ; "value is case insensitive")]
    #[test_case(Some("  true  "), true ; "surrounding whitespace is trimmed")]
    #[test_case(Some("false"), false ; "false keeps scheduling enabled")]
    #[test_case(Some("1"), false ; "one keeps scheduling enabled")]
    #[test_case(Some(""), false ; "empty keeps scheduling enabled")]
    #[test_case(Some("yes please"), false ; "unparseable keeps scheduling enabled")]
    #[serial]
    fn scheduler_disabled_by_env_only_honors_true(value: Option<&str>, expected: bool) {
        let _guard = env_lock::lock_env([(GOOSE_ACP_SCHEDULER_DISABLED_ENV, value)]);

        assert_eq!(scheduler_disabled_by_env(), expected);
    }

    #[tokio::test]
    #[serial]
    async fn scheduler_is_none_and_writes_nothing_when_disabled() {
        let data_dir = tempfile::tempdir().unwrap();
        let server = AcpServer::new(AcpServerFactoryConfig {
            builtins: Vec::new(),
            data_dir: data_dir.path().to_path_buf(),
            config_dir: data_dir.path().to_path_buf(),
            goose_platform: GoosePlatform::GooseCli,
            additional_source_roots: Vec::new(),
        });
        let _guard = env_lock::lock_env([(GOOSE_ACP_SCHEDULER_DISABLED_ENV, Some("true"))]);

        assert!(server.scheduler().await.unwrap().is_none());
        assert!(!data_dir.path().join("schedule.json").exists());
    }

    #[tokio::test]
    #[serial]
    async fn scheduler_is_built_when_enabled() {
        let data_dir = tempfile::tempdir().unwrap();
        let server = AcpServer::new(AcpServerFactoryConfig {
            builtins: Vec::new(),
            data_dir: data_dir.path().to_path_buf(),
            config_dir: data_dir.path().to_path_buf(),
            goose_platform: GoosePlatform::GooseCli,
            additional_source_roots: Vec::new(),
        });
        let _guard = env_lock::lock_env([(GOOSE_ACP_SCHEDULER_DISABLED_ENV, None::<&str>)]);

        assert!(server.scheduler().await.unwrap().is_some());
    }
}
