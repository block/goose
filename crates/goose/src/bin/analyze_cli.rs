//! CLI wrapper for the analyze platform extension.
//! Usage: cargo run -p goose --bin analyze_cli -- <path> [--focus <symbol>] [--depth <n>] [--follow <n>] [--force]

use clap::Parser;
use goose::agents::platform_extensions::analyze::{format, graph, AnalyzeClient};
use rayon::prelude::*;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "analyze_cli", about = "Ad-hoc code analysis via tree-sitter")]
struct Cli {
    /// File or directory path to analyze
    path: PathBuf,
    /// Symbol name to focus on (triggers call graph mode)
    #[arg(long)]
    focus: Option<String>,
    /// Directory recursion depth limit (default 3, 0=unlimited)
    #[arg(long, default_value_t = 3)]
    depth: u32,
    /// Call graph traversal depth (default 2, 0=definitions only)
    #[arg(long, default_value_t = 2)]
    follow: u32,
    /// Allow large outputs without size warning
    #[arg(long)]
    force: bool,
}

fn main() {
    let cli = Cli::parse();
    let path = if cli.path.is_absolute() {
        cli.path.clone()
    } else {
        std::env::current_dir().unwrap().join(&cli.path)
    };

    if !path.exists() {
        eprintln!("Error: path not found: {}", path.display());
        std::process::exit(1);
    }

    let output = if let Some(ref symbol) = cli.focus {
        // Focused mode: symbol call graph
        let files = if path.is_file() {
            vec![path.clone()]
        } else {
            AnalyzeClient::collect_files(&path, cli.depth)
        };
        let analyses: Vec<_> = files
            .par_iter()
            .filter_map(|f| AnalyzeClient::analyze_file(f))
            .collect();
        let root = if path.is_file() {
            path.parent().unwrap_or(&path)
        } else {
            &path
        };
        let g = graph::CallGraph::build(&analyses);
        format::format_focused(symbol, &g, cli.follow, analyses.len(), root)
    } else if path.is_file() {
        // Semantic mode: single file details
        match AnalyzeClient::analyze_file(&path) {
            Some(analysis) => {
                let root = path.parent().unwrap_or(&path);
                format::format_semantic(&analysis, root)
            }
            None => {
                eprintln!(
                    "Error: unsupported language or binary file: {}",
                    path.display()
                );
                std::process::exit(1);
            }
        }
    } else {
        // Structure mode: directory overview
        let files = AnalyzeClient::collect_files(&path, cli.depth);
        let total_files = files.len();
        let analyses: Vec<_> = files
            .par_iter()
            .filter_map(|f| AnalyzeClient::analyze_file(f))
            .collect();
        format::format_structure(&analyses, &path, cli.depth, total_files)
    };

    match format::check_size(&output, cli.force) {
        Ok(text) => print!("{text}"),
        Err(warning) => {
            eprintln!("{warning}");
            eprintln!("(use --force to see full output)");
            std::process::exit(2);
        }
    }
}
