use anyhow::{anyhow, Result};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::paths::Paths;
use crate::recipe::read_recipe_file_content::{
    expand_tilde_path, read_recipe_file, read_recipe_file_with_byte_limit, RecipeFile,
};
use crate::recipe::Recipe;
use crate::recipe::RECIPE_FILE_EXTENSIONS;

const GOOSE_RECIPE_PATH_ENV_VAR: &str = "GOOSE_RECIPE_PATH";

pub fn get_recipe_library_dir(is_global: bool) -> PathBuf {
    if is_global {
        Paths::config_dir().join("recipes")
    } else {
        env::current_dir().unwrap().join(".goose/recipes")
    }
}

fn local_recipe_dirs() -> Vec<PathBuf> {
    let mut local_dirs = vec![PathBuf::from(".")];

    if let Ok(recipe_path_env) = env::var(GOOSE_RECIPE_PATH_ENV_VAR) {
        let path_separator = if cfg!(windows) { ';' } else { ':' };
        local_dirs.extend(recipe_path_env.split(path_separator).map(PathBuf::from));
    }
    local_dirs.push(get_recipe_library_dir(true));
    local_dirs.push(get_recipe_library_dir(false));

    // Also scan .agents/recipes/ for consistency with the .agents/ convention
    if let Ok(cwd) = env::current_dir() {
        local_dirs.push(cwd.join(".agents/recipes"));
    }
    if let Some(home) = dirs::home_dir() {
        local_dirs.push(home.join(".agents/recipes"));
    }

    let mut dirs: Vec<PathBuf> = local_dirs
        .into_iter()
        .map(|dir| dir.canonicalize().unwrap_or(dir))
        .collect();
    dirs.sort();
    dirs.dedup();
    dirs
}

pub fn load_local_recipe_file(recipe_name: &str) -> Result<RecipeFile> {
    load_local_recipe_file_with_reader(recipe_name, |path| read_recipe_file(path))
}

pub fn load_local_recipe_file_with_byte_limit(
    recipe_name: &str,
    max_bytes: usize,
) -> Result<RecipeFile> {
    load_local_recipe_file_with_reader(recipe_name, |path| {
        read_recipe_file_with_byte_limit(path, max_bytes)
    })
}

pub struct BoundedLocalRecipeLoad {
    pub recipe_file: Result<RecipeFile>,
    pub consumed_bytes: usize,
}

pub fn load_local_recipe_file_with_byte_limit_and_consumption(
    recipe_name: &str,
    max_bytes: usize,
) -> BoundedLocalRecipeLoad {
    load_local_recipe_file_with_byte_limit_from_dirs(recipe_name, max_bytes, &local_recipe_dirs())
}

fn load_local_recipe_file_with_byte_limit_from_dirs(
    recipe_name: &str,
    max_bytes: usize,
    search_dirs: &[PathBuf],
) -> BoundedLocalRecipeLoad {
    if RECIPE_FILE_EXTENSIONS
        .iter()
        .any(|ext| recipe_name.ends_with(&format!(".{ext}")))
    {
        return read_bounded_local_candidate(Path::new(recipe_name), max_bytes);
    }

    if is_file_path(recipe_name) || is_file_name(recipe_name) {
        return BoundedLocalRecipeLoad {
            recipe_file: Err(anyhow!(
                "Recipe file {} is not a json or yaml file",
                recipe_name
            )),
            consumed_bytes: 0,
        };
    }

    let mut consumed_bytes = 0;
    for dir in search_dirs {
        for ext in RECIPE_FILE_EXTENSIONS {
            let remaining_bytes = max_bytes - consumed_bytes;
            let recipe_path = dir.join(format!("{recipe_name}.{ext}"));
            let candidate = read_bounded_local_candidate(&recipe_path, remaining_bytes);
            consumed_bytes += candidate.consumed_bytes;
            if candidate.recipe_file.is_ok() {
                return BoundedLocalRecipeLoad {
                    recipe_file: candidate.recipe_file,
                    consumed_bytes,
                };
            }
        }
    }

    let search_dirs_str = search_dirs
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(":");
    BoundedLocalRecipeLoad {
        recipe_file: Err(anyhow!(
            "ℹ️  Failed to retrieve {}.yaml or {}.json in {}",
            recipe_name,
            recipe_name,
            search_dirs_str
        )),
        consumed_bytes,
    }
}

fn read_bounded_local_candidate(path: &Path, max_bytes: usize) -> BoundedLocalRecipeLoad {
    let path = expand_tilde_path(path);
    let known_bytes = fs::symlink_metadata(&path)
        .ok()
        .filter(|metadata| metadata.is_file())
        .and_then(|metadata| usize::try_from(metadata.len()).ok())
        .filter(|file_bytes| *file_bytes <= max_bytes);
    let recipe_file = read_recipe_file_with_byte_limit(&path, max_bytes);
    let consumed_bytes = match &recipe_file {
        Ok(recipe_file) => recipe_file.content.len(),
        Err(_) => known_bytes.unwrap_or(0),
    };
    BoundedLocalRecipeLoad {
        recipe_file,
        consumed_bytes,
    }
}

fn load_local_recipe_file_with_reader(
    recipe_name: &str,
    read_file: impl Fn(&Path) -> Result<RecipeFile>,
) -> Result<RecipeFile> {
    if RECIPE_FILE_EXTENSIONS
        .iter()
        .any(|ext| recipe_name.ends_with(&format!(".{}", ext)))
    {
        let path = PathBuf::from(recipe_name);
        return read_file(&path);
    }

    if is_file_path(recipe_name) || is_file_name(recipe_name) {
        return Err(anyhow!(
            "Recipe file {} is not a json or yaml file",
            recipe_name
        ));
    }

    let search_dirs = local_recipe_dirs();
    for dir in &search_dirs {
        if let Ok(result) = load_recipe_file_from_dir(dir, recipe_name, &read_file) {
            return Ok(result);
        }
    }

    let search_dirs_str = search_dirs
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join(":");
    Err(anyhow!(
        "ℹ️  Failed to retrieve {}.yaml or {}.json in {}",
        recipe_name,
        recipe_name,
        search_dirs_str
    ))
}

pub fn list_local_recipes() -> Result<Vec<(PathBuf, Recipe)>> {
    let mut recipes = Vec::new();
    for dir in local_recipe_dirs() {
        if let Ok(dir_recipes) = scan_directory_for_recipes(&dir) {
            recipes.extend(dir_recipes);
        }
    }

    Ok(recipes)
}

fn is_file_path(recipe_name: &str) -> bool {
    recipe_name.contains('/')
        || recipe_name.contains('\\')
        || recipe_name.starts_with('~')
        || recipe_name.starts_with('.')
}

fn is_file_name(recipe_name: &str) -> bool {
    Path::new(recipe_name).extension().is_some()
}

fn load_recipe_file_from_dir(
    dir: &Path,
    recipe_name: &str,
    read_file: &impl Fn(&Path) -> Result<RecipeFile>,
) -> Result<RecipeFile> {
    for ext in RECIPE_FILE_EXTENSIONS {
        let recipe_path = dir.join(format!("{}.{}", recipe_name, ext));
        if let Ok(result) = read_file(&recipe_path) {
            return Ok(result);
        }
    }
    Err(anyhow!(format!(
        "No {}.yaml or {}.json recipe file found in directory: {}",
        recipe_name,
        recipe_name,
        dir.display()
    )))
}

fn scan_directory_for_recipes(dir: &Path) -> Result<Vec<(PathBuf, Recipe)>> {
    let mut recipes = Vec::new();

    if !dir.exists() || !dir.is_dir() {
        return Ok(recipes);
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(extension) = path.extension() {
                if RECIPE_FILE_EXTENSIONS.contains(&extension.to_string_lossy().as_ref()) {
                    match Recipe::from_file_path(&path) {
                        Ok(recipe) => recipes.push((path.clone(), recipe)),
                        Err(e) => {
                            let error_message = format!(
                                "Failed to load recipe from file {}: {}",
                                path.display(),
                                e
                            );
                            tracing::error!("{}", error_message);
                        }
                    }
                }
            }
        }
    }

    Ok(recipes)
}

fn generate_recipe_filename(title: &str, recipe_library_dir: &Path) -> PathBuf {
    let base_name = title
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace() || *c == '-')
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join("-");

    let filename = if base_name.is_empty() {
        "untitled-recipe".to_string()
    } else {
        base_name
    };

    let mut candidate = recipe_library_dir.join(format!("{}.yaml", filename));
    if !candidate.exists() {
        return candidate;
    }

    let mut counter = 1;
    loop {
        candidate = recipe_library_dir.join(format!("{}-{}.yaml", filename, counter));
        if !candidate.exists() {
            return candidate;
        }
        counter += 1;
    }
}

pub fn save_recipe_to_file(recipe: Recipe, file_path: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    let recipe_library_dir = get_recipe_library_dir(true);

    let file_path_value = match file_path {
        Some(path) => path,
        None => generate_recipe_filename(&recipe.title, &recipe_library_dir),
    };

    if let Some(parent) = file_path_value.parent() {
        fs::create_dir_all(parent)?;
    }

    let yaml_content = recipe.to_yaml()?;
    fs::write(&file_path_value, yaml_content)?;
    Ok(file_path_value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_extensionless_search_charges_failed_candidates() {
        let directory = tempfile::tempdir().unwrap();
        let invalid_yaml = [0xff; 8];
        let valid_json = br#"{"title":"valid"}"#;
        fs::write(directory.path().join("candidate.yaml"), invalid_yaml).unwrap();
        fs::write(directory.path().join("candidate.json"), valid_json).unwrap();
        let budget = invalid_yaml.len() + valid_json.len();

        let loaded = load_local_recipe_file_with_byte_limit_from_dirs(
            "candidate",
            budget,
            &[directory.path().to_path_buf()],
        );

        assert_eq!(loaded.consumed_bytes, budget);
        assert_eq!(loaded.recipe_file.unwrap().content.as_bytes(), valid_json);

        let exhausted = load_local_recipe_file_with_byte_limit_from_dirs(
            "candidate",
            invalid_yaml.len(),
            &[directory.path().to_path_buf()],
        );
        assert_eq!(exhausted.consumed_bytes, invalid_yaml.len());
        assert!(exhausted.recipe_file.is_err());
    }

    #[cfg(unix)]
    #[test]
    fn bounded_tilde_paths_share_read_and_accounting_resolution() {
        let home = tempfile::tempdir().unwrap();
        let home_path = home.path().to_string_lossy().into_owned();
        let _guard = env_lock::lock_env([("HOME", Some(home_path.as_str()))]);
        let invalid_content = [0xff; 8];
        fs::write(home.path().join("invalid.yaml"), invalid_content).unwrap();
        let valid_content = "title: valid";
        fs::write(home.path().join("valid.yaml"), valid_content).unwrap();

        let invalid = read_bounded_local_candidate(Path::new("~/invalid.yaml"), 32);
        assert!(invalid.recipe_file.is_err());
        assert_eq!(invalid.consumed_bytes, invalid_content.len());

        let valid = read_bounded_local_candidate(Path::new("~/valid.yaml"), 32);
        assert_eq!(valid.consumed_bytes, valid_content.len());
        assert_eq!(valid.recipe_file.unwrap().content, valid_content);
    }
}
