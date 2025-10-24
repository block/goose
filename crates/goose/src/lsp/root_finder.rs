use std::path::{Path, PathBuf};

pub fn find_lsp_root(file_path: &Path, patterns: &[String], workspace_root: &Path) -> PathBuf {
    let mut current_dir = file_path.parent().unwrap_or(file_path);

    while current_dir.starts_with(workspace_root) {
        for pattern in patterns {
            let candidate = current_dir.join(pattern);
            if candidate.exists() {
                return current_dir.to_path_buf();
            }
        }

        match current_dir.parent() {
            Some(parent) => current_dir = parent,
            None => break,
        }
    }

    workspace_root.to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_find_lsp_root_with_pattern() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();

        let project_dir = workspace.join("project");
        fs::create_dir(&project_dir).unwrap();
        fs::write(project_dir.join("package.json"), "{}").unwrap();

        let src_dir = project_dir.join("src");
        fs::create_dir(&src_dir).unwrap();
        let file = src_dir.join("main.ts");
        fs::write(&file, "").unwrap();

        let root = find_lsp_root(&file, &["package.json".to_string()], workspace);

        assert_eq!(root, project_dir);
    }

    #[test]
    fn test_find_lsp_root_defaults_to_workspace() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();

        let file = workspace.join("file.ts");
        fs::write(&file, "").unwrap();

        let root = find_lsp_root(&file, &["package.json".to_string()], workspace);

        assert_eq!(root, workspace);
    }
}
