//! CLI wrapper for the analyze platform extension.
//! Usage: cargo run -p goose --bin analyze_cli -- <path> [--focus <symbol>] [--depth <n>] [--follow <n>] [--force]

use clap::Parser;
use goose::agents::platform_extensions::analyze::{format, graph, parser};
use ignore::WalkBuilder;
use rayon::prelude::*;
use std::path::{Path, PathBuf};

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

fn analyze_file(path: &Path) -> Option<parser::FileAnalysis> {
    let source = std::fs::read_to_string(path).ok()?;
    parser::Parser::new().analyze_file(path, &source)
}

fn collect_files(dir: &Path, max_depth: u32) -> Vec<PathBuf> {
    let mut builder = WalkBuilder::new(dir);
    if max_depth > 0 {
        builder.max_depth(Some(max_depth as usize));
    }
    builder
        .build()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_some_and(|ft| ft.is_file()))
        .map(|e| e.into_path())
        .collect()
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
            collect_files(&path, cli.depth)
        };
        let analyses: Vec<_> = files.par_iter().filter_map(|f| analyze_file(f)).collect();
        let g = graph::CallGraph::build(&analyses);
        format::format_focused(symbol, &g, cli.follow, analyses.len())
    } else if path.is_file() {
        // Semantic mode: single file details
        match analyze_file(&path) {
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
        let files = collect_files(&path, cli.depth);
        let total_files = files.len();
        let analyses: Vec<_> = files.par_iter().filter_map(|f| analyze_file(f)).collect();
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
