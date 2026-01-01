//! Codebase exploration tools.
//!
//! This module provides the `map` tool for understanding codebase structure.

pub mod map;

pub use map::{MapParams, MapTool};

use std::path::Path;

/// Check if a path should be ignored during traversal.
pub fn should_ignore(path: &Path) -> bool {
    let name = match path.file_name().and_then(|n| n.to_str()) {
        Some(n) => n,
        None => return false,
    };

    // Hidden files/directories
    if name.starts_with('.') {
        return true;
    }

    // Common ignore patterns
    matches!(
        name,
        "node_modules"
            | "target"
            | "build"
            | "dist"
            | "__pycache__"
            | ".git"
            | "vendor"
            | "venv"
            | ".venv"
            | "env"
            | ".env"
            | "coverage"
            | ".coverage"
            | "htmlcov"
            | ".pytest_cache"
            | ".mypy_cache"
            | ".tox"
            | "eggs"
            | "*.egg-info"
            | ".cargo"
            | "Pods"
            | ".build"
            | "DerivedData"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_ignore() {
        assert!(should_ignore(Path::new(".git")));
        assert!(should_ignore(Path::new("node_modules")));
        assert!(should_ignore(Path::new("target")));
        assert!(!should_ignore(Path::new("src")));
        assert!(!should_ignore(Path::new("main.rs")));
    }
}
