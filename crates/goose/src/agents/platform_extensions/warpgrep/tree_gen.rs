use std::path::Path;

use ignore::WalkBuilder;

const MAX_DEPTH: usize = 8;
const MAX_FILES: usize = 5000;

/// Known binary file extensions to skip in the repo structure listing.
const BINARY_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "bmp", "ico", "svg", "webp", "mp3", "mp4", "wav", "avi", "mov",
    "mkv", "flac", "ogg", "woff", "woff2", "ttf", "eot", "otf", "zip", "tar", "gz", "bz2", "xz",
    "7z", "rar", "jar", "war", "ear", "dll", "so", "dylib", "exe", "bin", "obj", "o", "a", "lib",
    "pyc", "pyo", "class", "wasm", "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "sqlite",
    "db", "DS_Store",
];

pub fn generate_repo_structure(root: &Path) -> String {
    let mut builder = WalkBuilder::new(root);
    builder.git_ignore(true);
    builder.git_exclude(true);
    builder.git_global(true);
    builder.require_git(false);
    builder.ignore(true);
    builder.hidden(true);
    builder.max_depth(Some(MAX_DEPTH));

    let mut paths = Vec::new();
    for entry in builder.build().flatten() {
        if paths.len() >= MAX_FILES {
            break;
        }

        let path = entry.path();
        if path == root {
            continue;
        }

        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }

        if is_binary_extension(path) {
            continue;
        }

        if let Ok(relative) = path.strip_prefix(root) {
            paths.push(relative.to_string_lossy().into_owned());
        }
    }

    paths.sort();
    paths.join("\n")
}

fn is_binary_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| BINARY_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn generates_flat_file_listing() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src/main.rs"), "fn main() {}").unwrap();
        fs::write(dir.path().join("README.md"), "# Hello").unwrap();

        let output = generate_repo_structure(dir.path());
        assert!(output.contains("src/main.rs"));
        assert!(output.contains("README.md"));
    }

    #[test]
    fn skips_binary_files() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("code.rs"), "fn main() {}").unwrap();
        fs::write(dir.path().join("image.png"), [0u8; 10]).unwrap();
        fs::write(dir.path().join("archive.zip"), [0u8; 10]).unwrap();

        let output = generate_repo_structure(dir.path());
        assert!(output.contains("code.rs"));
        assert!(!output.contains("image.png"));
        assert!(!output.contains("archive.zip"));
    }

    #[test]
    fn respects_gitignore() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join(".gitignore"), "target/\n").unwrap();
        fs::create_dir_all(dir.path().join("target")).unwrap();
        fs::write(dir.path().join("target/debug.rs"), "fn d() {}").unwrap();
        fs::write(dir.path().join("lib.rs"), "pub fn lib() {}").unwrap();

        let output = generate_repo_structure(dir.path());
        assert!(output.contains("lib.rs"));
        assert!(!output.contains("target"));
    }

    #[test]
    fn empty_directory_returns_empty_string() {
        let dir = tempdir().unwrap();
        let output = generate_repo_structure(dir.path());
        assert!(output.is_empty());
    }
}
