// Regression tests for https://github.com/aaif-goose/goose/issues/11138
//
// `GOOSE_CLI_SHOW_THINKING` is documented as "set to any value to enable"
// (documentation/docs/guides/environment-variables.md), and `GOOSE_CLI_SHOW_COST`
// is documented as accepting `"1"` / `"true"`. The reporter set
// `GOOSE_CLI_SHOW_THINKING=1` and saw no thinking output in the CLI, while the
// Desktop app showed the reasoning for the same session.
//
// Root cause: `Config::get_param::<bool>` reads the env value through
// `parse_env_value`, which runs `serde_json::from_str("1")` first and yields a
// JSON *number*. `serde_json::from_value::<bool>(Number(1))` then fails, so
// `get_param::<bool>` returns an error and the CLI render gate
// (`crates/goose-cli/src/session/output.rs::should_show_thinking`) falls back to
// `.unwrap_or(false)` — the documented `1` value is silently ignored.
use goose::config::Config;

#[test]
fn show_thinking_env_1_enables_thinking() {
    std::env::set_var("GOOSE_CLI_SHOW_THINKING", "1");
    let enabled = Config::global()
        .get_param::<bool>("GOOSE_CLI_SHOW_THINKING")
        .unwrap_or(false);
    std::env::remove_var("GOOSE_CLI_SHOW_THINKING");
    assert!(
        enabled,
        "GOOSE_CLI_SHOW_THINKING=1 should enable thinking output in the CLI"
    );
}

#[test]
fn show_cost_env_1_enables_cost() {
    // GOOSE_CLI_SHOW_COST is documented as accepting "1"/"true".
    std::env::set_var("GOOSE_CLI_SHOW_COST", "1");
    let enabled = Config::global()
        .get_param::<bool>("GOOSE_CLI_SHOW_COST")
        .unwrap_or(false);
    std::env::remove_var("GOOSE_CLI_SHOW_COST");
    assert!(enabled, "GOOSE_CLI_SHOW_COST=1 should enable cost display");
}

#[test]
fn bool_env_0_disables() {
    std::env::set_var("GOOSE_TEST_BOOL_0", "0");
    let enabled = Config::global()
        .get_param::<bool>("GOOSE_TEST_BOOL_0")
        .unwrap_or(true);
    std::env::remove_var("GOOSE_TEST_BOOL_0");
    assert!(!enabled, "GOOSE_TEST_BOOL_0=0 should disable the flag");
}

#[test]
fn bool_env_true_unchanged() {
    // The literal "true" value already worked before the fix; pin it so a future
    // regression in the coercion path is caught.
    std::env::set_var("GOOSE_TEST_BOOL_TRUE", "true");
    let enabled = Config::global()
        .get_param::<bool>("GOOSE_TEST_BOOL_TRUE")
        .unwrap_or(false);
    std::env::remove_var("GOOSE_TEST_BOOL_TRUE");
    assert!(enabled, "GOOSE_TEST_BOOL_TRUE=true should enable the flag");
}
