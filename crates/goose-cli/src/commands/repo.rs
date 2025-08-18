use anyhow::Result;
use std::fs::File;
use std::path::Path;

/// Thin CLI wrapper delegating to the core repo index implementation (`goose::repo_index`).
/// Keeps the CLI free from Tree-sitter details; feature-gated logic lives in the core crate.
pub fn index_repository_with_args(root_path: &str, output_file: &str) -> Result<()> {
    let root = Path::new(root_path);
    let mut file = File::create(output_file)?;
    let opts = goose::repo_index::RepoIndexOptions::builder()
        .root(root)
        .output_writer(&mut file)
    // in future we could plumb CLI flags for filtering/progress here
        .build();
    let stats = goose::repo_index::index_repository(opts)?;
    eprintln!("Indexed {} files / {} entities", stats.files_indexed, stats.entities_indexed);
    Ok(())
}
