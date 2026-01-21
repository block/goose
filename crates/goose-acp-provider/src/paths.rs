use etcetera::{choose_app_strategy, AppStrategy, AppStrategyArgs};
use std::path::PathBuf;

pub(crate) fn data_dir() -> PathBuf {
    if let Ok(test_root) = std::env::var("GOOSE_PATH_ROOT") {
        return PathBuf::from(test_root).join("data");
    }

    // Duplicated from crates/goose/src/config/paths.rs to avoid a crate dependency cycle.
    let strategy = choose_app_strategy(AppStrategyArgs {
        top_level_domain: "Block".to_string(),
        author: "Block".to_string(),
        app_name: "goose".to_string(),
    })
    .expect("goose requires a home dir");

    strategy.data_dir()
}
