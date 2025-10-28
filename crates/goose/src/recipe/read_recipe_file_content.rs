use anyhow::{anyhow, Result};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct RecipeFile {
    pub content: String,
    pub parent_dir: PathBuf,
    pub file_path: PathBuf,
}

pub fn read_recipe_file<P: AsRef<Path>>(recipe_path: P) -> Result<RecipeFile> {
    let raw_path = recipe_path.as_ref();
    let path = convert_path_with_tilde_expansion(raw_path);

    let content = fs::read_to_string(&path)
        .map_err(|e| anyhow!("Failed to read recipe file {}: {}", path.display(), e))?;

    let canonical = path.canonicalize().map_err(|e| {
        anyhow!(
            "Failed to resolve absolute path for {}: {}",
            path.display(),
            e
        )
    })?;

    let parent_dir = canonical
        .parent()
        .ok_or_else(|| anyhow!("Resolved path has no parent: {}", canonical.display()))?
        .to_path_buf();

    Ok(RecipeFile {
        content,
        parent_dir,
        file_path: canonical,
    })
}

fn convert_path_with_tilde_expansion(path: &Path) -> PathBuf {
    if let Some(path_str) = path.to_str() {
        // Handle exact "~" (Windows only to avoid changing behavior on Unix)
        if cfg!(windows) && path_str == "~" {
            if let Some(home_dir) = dirs::home_dir() {
                return home_dir;
            }
        }
        // Handle Unix-style "~/..."
        if let Some(stripped) = path_str.strip_prefix("~/") {
            if let Some(home_dir) = dirs::home_dir() {
                return home_dir.join(stripped);
            }
        }
        // Handle Windows-style "~\\..." (Windows only)
        #[cfg(windows)]
        if let Some(stripped) = path_str.strip_prefix("~\\") {
            if let Some(home_dir) = dirs::home_dir() {
                return home_dir.join(stripped);
            }
        }
    }
    PathBuf::from(path)
}

pub fn read_parameter_file_content<P: AsRef<Path>>(file_path: P) -> Result<String> {
    let raw_path = file_path.as_ref();
    let path = convert_path_with_tilde_expansion(raw_path);

    let content = fs::read_to_string(&path)
        .map_err(|e| anyhow!("Failed to read parameter file {}: {}", path.display(), e))?;

    Ok(content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_read_parameter_file_content_success() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test_file.txt");
        let content = "Hello World\nSecond line\n    Third line";
        std::fs::write(&file_path, content).unwrap();

        let result = read_parameter_file_content(&file_path);
        assert!(result.is_ok());

        let expected = "Hello World\nSecond line\n    Third line";
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_read_parameter_file_content_nonexistent_file() {
        let result = read_parameter_file_content("/nonexistent/path/file.txt");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to read parameter file"));
    }
}
