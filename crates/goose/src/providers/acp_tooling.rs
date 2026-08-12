use crate::config::search_path::SearchPaths;
use anyhow::Result;
use std::path::PathBuf;

pub fn acp_adapter_installed(command: &str) -> bool {
    resolve_acp_command(command).is_ok()
}

pub fn resolved_acp_command(command: &str) -> Result<PathBuf> {
    resolve_acp_command(command)
}

fn resolve_acp_command(command: &str) -> Result<PathBuf> {
    SearchPaths::builder().with_npm().resolve(command)
}
