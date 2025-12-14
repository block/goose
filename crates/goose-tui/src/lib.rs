pub mod app;
pub mod at_mention;
pub mod components;
pub mod headless;
pub mod hidden_blocks;
pub mod services;
pub mod state;
pub mod tui;
pub mod utils;

pub use utils::DEFAULT_CONTEXT_LIMIT;

pub mod analysis_target {
    use std::path::{Path, PathBuf};

    const CWD_ANALYSIS_DEPTH: u32 = 3;

    pub fn detect_analysis_target(cwd: &Path) -> (PathBuf, u32) {
        let monorepo_markers = [
            "pnpm-workspace.yaml",
            "lerna.json",
            "nx.json",
            "turbo.json",
            "rush.json",
            "go.work",
        ];

        let is_rust_workspace = std::fs::read_to_string(cwd.join("Cargo.toml"))
            .map(|content| content.contains("[workspace]"))
            .unwrap_or(false);

        let is_monorepo =
            is_rust_workspace || monorepo_markers.iter().any(|m| cwd.join(m).exists());

        if is_monorepo {
            let monorepo_dirs = ["crates", "packages", "apps", "libs", "cmd", "internal"];
            for dir in monorepo_dirs {
                let path = cwd.join(dir);
                if path.is_dir() {
                    return (path, CWD_ANALYSIS_DEPTH);
                }
            }
            return (cwd.to_path_buf(), 1);
        }

        let source_dirs = ["src", "lib", "app", "cmd", "Sources", "internal"];
        for dir in source_dirs {
            let path = cwd.join(dir);
            if path.is_dir() {
                return (path, CWD_ANALYSIS_DEPTH);
            }
        }

        (cwd.to_path_buf(), 1)
    }
}
