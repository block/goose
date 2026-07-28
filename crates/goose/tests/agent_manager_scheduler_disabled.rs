//! Regression: the process-global `AgentManager` must honor the scheduler
//! kill-switch.
//!
//! `AgentManager::instance()` used to construct a `Scheduler` unconditionally,
//! so a host that asked for no scheduling still got cron polling and
//! `schedule.json` mutation as soon as anything touched the singleton. See the
//! sibling `orchestrator_scheduler_disabled` test for the extension path that
//! makes this reachable from inside an ACP process.
//!
//! This lives in its own integration-test binary on purpose: `AGENT_MANAGER` is
//! a process-global `OnceCell`, so the first `instance()` call in a process
//! decides what every later caller sees. A second test in this binary would
//! observe this test's singleton rather than building its own.

use goose::execution::manager::AgentManager;
use goose::scheduler::GOOSE_ACP_SCHEDULER_DISABLED_ENV;

/// A job left with `currently_running: true` is stale running state, which an
/// enabled scheduler rewrites during startup (`Scheduler::load_jobs_from_storage`).
/// That is what makes this file discriminating: an unchanged file proves the
/// read never happened, rather than proving nothing needed changing.
///
/// Asserting instead that no `schedule.json` was *created* would prove nothing
/// here — an enabled scheduler also writes no file when none exists.
const STALE_RUNNING_JOB: &str = r#"[{"id":"sentinel","source":"/nonexistent/sentinel.yaml","cron":"0 0 0 * * *","currently_running":true}]"#;

#[tokio::test]
async fn global_agent_manager_has_no_scheduler_and_leaves_schedule_file_untouched() {
    let root = tempfile::tempdir().unwrap();
    // `Paths::data_dir()` is `$GOOSE_PATH_ROOT/data`, and the root is silently
    // ignored unless it is absolute — an ignored root would point this test at
    // the developer's real schedule file.
    let data_dir = root.path().join("data");
    std::fs::create_dir_all(&data_dir).unwrap();
    assert!(
        root.path().is_absolute(),
        "GOOSE_PATH_ROOT is ignored unless absolute"
    );

    let schedule_file = data_dir.join("schedule.json");
    std::fs::write(&schedule_file, STALE_RUNNING_JOB).unwrap();
    let modified_before = std::fs::metadata(&schedule_file)
        .unwrap()
        .modified()
        .unwrap();

    let _guard = env_lock::lock_env([
        (GOOSE_ACP_SCHEDULER_DISABLED_ENV, Some("true")),
        ("GOOSE_DISABLE_KEYRING", Some("true")),
        ("GOOSE_PATH_ROOT", root.path().to_str()),
    ]);

    let manager = AgentManager::instance()
        .await
        .expect("the global manager is constructible without a scheduler");

    assert!(
        manager.scheduler().is_none(),
        "the global manager must not build a scheduler when the switch is set"
    );
    assert_eq!(
        std::fs::read_to_string(&schedule_file).unwrap(),
        STALE_RUNNING_JOB,
        "a disabled runtime must not rewrite the schedule file"
    );
    assert_eq!(
        std::fs::metadata(&schedule_file)
            .unwrap()
            .modified()
            .unwrap(),
        modified_before,
        "a disabled runtime must not even touch the schedule file"
    );
}
