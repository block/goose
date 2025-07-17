use anyhow::{anyhow, Result};
use std::fs;
use std::path::{Path, PathBuf};
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
        if let Some(stripped) = path_str.strip_prefix("~/") {
            if let Some(home_dir) = dirs::home_dir() {
                return home_dir.join(stripped);
            }
        }
    }
    PathBuf::from(path)
}
