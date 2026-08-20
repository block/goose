use anyhow::{anyhow, Result};
use console::style;
use goose::recipe::template_recipe::parse_recipe_content;
use goose::recipe::RECIPE_FILE_EXTENSIONS;
use serde::{Deserialize, Serialize};

use goose::recipe::read_recipe_file_content::RecipeFile;
use goose::subprocess::{git_command, SubprocessExt};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::io::{self, Read};

use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::process::Stdio;
use tar::Archive;

#[derive(Clone, Copy)]
struct ArchiveLimits {
    entries: usize,
    path_bytes: usize,
    total_path_bytes: usize,
    inodes: usize,
    overhead_bytes: usize,
}

const ARCHIVE_LIMITS: ArchiveLimits = ArchiveLimits {
    entries: 1024,
    path_bytes: 4096,
    total_path_bytes: 1024 * 1024,
    inodes: 4096,
    overhead_bytes: 4 * 1024 * 1024,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeInfo {
    pub name: String,
    pub source: RecipeSource,
    pub path: String,
    pub title: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecipeSource {
    Local,
    GitHub,
}

pub const GOOSE_RECIPE_GITHUB_REPO_CONFIG_KEY: &str = "GOOSE_RECIPE_GITHUB_REPO";
pub fn retrieve_recipe_from_github(
    recipe_name: &str,
    recipe_repo_full_name: &str,
) -> Result<RecipeFile> {
    Ok(
        retrieve_recipe_from_github_with_optional_limit(recipe_name, recipe_repo_full_name, None)?
            .recipe_file,
    )
}

pub(crate) fn retrieve_recipe_from_github_with_byte_limit(
    recipe_name: &str,
    recipe_repo_full_name: &str,
    max_bytes: usize,
) -> Result<(RecipeFile, usize)> {
    let retrieved = retrieve_recipe_from_github_with_optional_limit(
        recipe_name,
        recipe_repo_full_name,
        Some(max_bytes),
    )?;
    let payload_bytes = retrieved
        .payload_bytes
        .ok_or_else(|| anyhow!("Bounded recipe download did not report its payload size"))?;
    Ok((retrieved.recipe_file, payload_bytes))
}

struct RetrievedRecipe {
    recipe_file: RecipeFile,
    payload_bytes: Option<usize>,
}

fn retrieve_recipe_from_github_with_optional_limit(
    recipe_name: &str,
    recipe_repo_full_name: &str,
    max_bytes: Option<usize>,
) -> Result<RetrievedRecipe> {
    println!(
        "📦 Looking for recipe \"{}\" in github repo: {}",
        recipe_name, recipe_repo_full_name
    );
    ensure_gh_authenticated()?;
    let max_attempts = 2;
    let mut last_err = None;

    for attempt in 1..=max_attempts {
        let download = match max_bytes {
            Some(max_bytes) => clone_and_download_recipe_with_byte_limit(
                recipe_name,
                recipe_repo_full_name,
                max_bytes,
            )
            .map(|(download_dir, payload_bytes)| (download_dir, Some(payload_bytes))),
            None => clone_and_download_recipe(recipe_name, recipe_repo_full_name)
                .map(|download_dir| (download_dir, None)),
        };
        match download {
            Ok((download_dir, payload_bytes)) => match read_recipe_file(&download_dir, max_bytes) {
                Ok((content, recipe_file_local_path)) => {
                    return Ok(RetrievedRecipe {
                        recipe_file: RecipeFile {
                            content,
                            parent_dir: download_dir.clone(),
                            file_path: recipe_file_local_path,
                        },
                        payload_bytes,
                    });
                }
                Err(err) => {
                    let _ = fs::remove_dir_all(download_dir);
                    return Err(err);
                }
            },
            Err(err) => {
                last_err = Some(err);
            }
        }
        if attempt < max_attempts {
            clean_cloned_dirs(recipe_repo_full_name)?;
        }
    }
    Err(last_err.unwrap_or_else(|| anyhow::anyhow!("Unknown error occurred")))
}

fn clean_cloned_dirs(recipe_repo_full_name: &str) -> anyhow::Result<()> {
    let local_repo_path = get_local_repo_path(&env::temp_dir(), recipe_repo_full_name)?;
    if local_repo_path.exists() {
        fs::remove_dir_all(&local_repo_path)?;
    }
    Ok(())
}
fn read_recipe_file(download_dir: &Path, max_bytes: Option<usize>) -> Result<(String, PathBuf)> {
    for ext in RECIPE_FILE_EXTENSIONS {
        let candidate_file_path = download_dir.join(format!("recipe.{}", ext));
        if candidate_file_path.exists() {
            let content = match max_bytes {
                Some(max_bytes) => read_utf8_with_byte_limit(&candidate_file_path, max_bytes)?,
                None => fs::read_to_string(&candidate_file_path)?,
            };
            println!(
                "⬇️  Retrieved recipe file: {}",
                candidate_file_path
                    .strip_prefix(download_dir)
                    .unwrap()
                    .display()
            );
            return Ok((content, candidate_file_path));
        }
    }

    Err(anyhow::anyhow!(
        "No recipe file found in {} (looked for extensions: {:?})",
        download_dir.display(),
        RECIPE_FILE_EXTENSIONS
    ))
}

fn read_utf8_with_byte_limit(path: &Path, max_bytes: usize) -> Result<String> {
    let file = fs::File::open(path)?;
    if file.metadata()?.len() > max_bytes as u64 {
        return Err(anyhow!("Recipe file exceeds the {max_bytes}-byte limit"));
    }
    let read_size = max_bytes
        .checked_add(1)
        .ok_or_else(|| anyhow!("Recipe byte limit is too large"))?;
    let mut bytes = Vec::new();
    file.take(read_size as u64).read_to_end(&mut bytes)?;
    if bytes.len() > max_bytes {
        return Err(anyhow!("Recipe file exceeds the {max_bytes}-byte limit"));
    }
    Ok(String::from_utf8(bytes)?)
}

fn clone_and_download_recipe(recipe_name: &str, recipe_repo_full_name: &str) -> Result<PathBuf> {
    let local_repo_path = ensure_repo_cloned(recipe_repo_full_name)?;
    fetch_origin(&local_repo_path)?;
    get_folder_from_github(&local_repo_path, recipe_name, recipe_repo_full_name)
}

fn clone_and_download_recipe_with_byte_limit(
    recipe_name: &str,
    recipe_repo_full_name: &str,
    max_bytes: usize,
) -> Result<(PathBuf, usize)> {
    let local_repo_path = ensure_repo_cloned(recipe_repo_full_name)?;
    fetch_origin(&local_repo_path)?;
    get_folder_from_github_with_byte_limit(
        &local_repo_path,
        recipe_name,
        recipe_repo_full_name,
        max_bytes,
    )
}

pub fn ensure_gh_authenticated() -> Result<()> {
    // Check authentication status
    let status = Command::new("gh")
        .args(["auth", "status"])
        .set_no_window()
        .status()
        .map_err(|_| {
            anyhow::anyhow!("Failed to run `gh auth status`. Make sure you have `gh` installed.")
        })?;

    if status.success() {
        return Ok(());
    }
    println!("GitHub CLI is not authenticated. Launching `gh auth login`...");
    // Run `gh auth login` interactively
    let login_status = Command::new("gh")
        .args(["auth", "login", "--git-protocol", "https"])
        .status()
        .map_err(|_| anyhow::anyhow!("Failed to run `gh auth login`"))?;

    if !login_status.success() {
        Err(anyhow::anyhow!("Failed to authenticate using GitHub CLI."))
    } else {
        Ok(())
    }
}

fn temp_child_name(name: &str) -> Result<String> {
    if !name.is_empty() && name.trim_end_matches([' ', '.']).is_empty() {
        return Err(anyhow!(
            "Recipe name does not map to a safe temporary directory"
        ));
    }

    let mut child = String::with_capacity(name.len());
    for ch in name.chars() {
        match ch {
            '/' | '\\' => child.push_str("__"),
            ch if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' => {
                child.push(ch)
            }
            _ => child.push('_'),
        }
    }

    if child.is_empty() {
        child.push('_');
    }

    if child.trim_end_matches([' ', '.']) != child {
        return Err(anyhow!(
            "Recipe name does not map to a safe temporary directory"
        ));
    }

    let mut components = Path::new(&child).components();
    match (components.next(), components.next()) {
        (Some(Component::Normal(_)), None) => Ok(child),
        _ => Err(anyhow!(
            "Recipe name does not map to a safe temporary directory"
        )),
    }
}

#[cfg(test)]
fn temp_child_path(parent: &Path, name: &str) -> Result<PathBuf> {
    Ok(parent.join(temp_child_name(name)?))
}

fn unique_temp_child_path(
    parent: &Path,
    recipe_name: &str,
    recipe_repo_full_name: &str,
) -> Result<PathBuf> {
    hashed_temp_child_path(parent, recipe_name, &[recipe_repo_full_name, recipe_name])
}

fn hashed_temp_child_path(parent: &Path, name: &str, identity: &[&str]) -> Result<PathBuf> {
    let child = temp_child_name(name)?;
    let visible_child = child.chars().take(160).collect::<String>();
    let mut hasher = Sha256::new();
    for field in identity {
        hasher.update((field.len() as u64).to_le_bytes());
        hasher.update(field.as_bytes());
    }
    let digest = hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Ok(parent.join(format!("{visible_child}-{digest}")))
}

fn clean_unique_temp_child_path(
    parent: &Path,
    recipe_name: &str,
    recipe_repo_full_name: &str,
) -> Result<PathBuf> {
    let output_dir = unique_temp_child_path(parent, recipe_name, recipe_repo_full_name)?;
    if output_dir.exists() {
        fs::remove_dir_all(&output_dir)?;
    }
    Ok(output_dir)
}

fn get_local_repo_path(
    local_repo_parent_path: &Path,
    recipe_repo_full_name: &str,
) -> Result<PathBuf> {
    let (owner, repo_name) = recipe_repo_full_name
        .split_once('/')
        .ok_or_else(|| anyhow::anyhow!("Invalid repository name format"))?;
    let local_repo_path = hashed_temp_child_path(
        local_repo_parent_path,
        &format!("{owner}/{repo_name}"),
        &[recipe_repo_full_name],
    )?;
    Ok(local_repo_path)
}

fn ensure_repo_cloned(recipe_repo_full_name: &str) -> Result<PathBuf> {
    let local_repo_parent_path = env::temp_dir();
    if !local_repo_parent_path.exists() {
        std::fs::create_dir_all(local_repo_parent_path.clone())?;
    }
    let local_repo_path = get_local_repo_path(&local_repo_parent_path, recipe_repo_full_name)?;

    if local_repo_path.join(".git").exists() {
        Ok(local_repo_path)
    } else {
        let error_message: String = format!("Failed to clone repo: {}", recipe_repo_full_name);
        let status = Command::new("gh")
            .args(["repo", "clone", recipe_repo_full_name])
            .arg(&local_repo_path)
            .current_dir(local_repo_parent_path.clone())
            .set_no_window()
            .status()
            .map_err(|_: std::io::Error| anyhow::anyhow!(error_message.clone()))?;

        if status.success() {
            Ok(local_repo_path)
        } else {
            Err(anyhow::anyhow!(error_message))
        }
    }
}

fn fetch_origin(local_repo_path: &Path) -> Result<()> {
    let error_message: String = format!("Failed to fetch at {}", local_repo_path.to_str().unwrap());
    let status = git_command()
        .args(["fetch", "origin"])
        .current_dir(local_repo_path)
        .set_no_window()
        .status()
        .map_err(|_| anyhow::anyhow!(error_message.clone()))?;

    if status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!(error_message))
    }
}

fn get_folder_from_github(
    local_repo_path: &Path,
    recipe_name: &str,
    recipe_repo_full_name: &str,
) -> Result<PathBuf> {
    let ref_and_path = format!("origin/main:{}", recipe_name);
    let output_dir =
        clean_unique_temp_child_path(&env::temp_dir(), recipe_name, recipe_repo_full_name)?;
    fs::create_dir_all(&output_dir)?;

    let archive_output = git_command()
        .args(["archive", &ref_and_path])
        .current_dir(local_repo_path)
        .stdout(Stdio::piped())
        .set_no_window()
        .spawn()?;

    let stdout = archive_output
        .stdout
        .ok_or_else(|| anyhow::anyhow!("Failed to capture stdout from git archive"))?;

    let mut archive = Archive::new(stdout);
    archive.unpack(&output_dir)?;
    list_files(&output_dir)?;

    Ok(output_dir)
}

fn get_folder_from_github_with_byte_limit(
    local_repo_path: &Path,
    recipe_name: &str,
    recipe_repo_full_name: &str,
    max_bytes: usize,
) -> Result<(PathBuf, usize)> {
    let ref_and_path = format!("origin/main:{recipe_name}");
    let output_dir =
        clean_unique_temp_child_path(&env::temp_dir(), recipe_name, recipe_repo_full_name)?;
    fs::create_dir_all(&output_dir)?;

    let mut child = git_command()
        .args(["archive", &ref_and_path])
        .current_dir(local_repo_path)
        .stdout(Stdio::piped())
        .set_no_window()
        .spawn()?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow!("Failed to capture stdout from git archive"))?;
    let extraction = extract_bounded_archive(stdout, &output_dir, max_bytes);
    if extraction.is_err() {
        let _ = child.kill();
    }
    let status = child.wait();
    let extraction = match (extraction, status) {
        (Ok(payload_bytes), Ok(status)) if status.success() => Ok(payload_bytes),
        (Ok(_), Ok(_)) => Err(anyhow!("Failed to archive recipe bundle {recipe_name}")),
        (Err(err), _) => Err(err),
        (_, Err(err)) => Err(err.into()),
    };
    if extraction.is_err() {
        let _ = fs::remove_dir_all(&output_dir);
    }
    let payload_bytes = extraction?;
    if let Err(err) = list_files(&output_dir) {
        let _ = fs::remove_dir_all(&output_dir);
        return Err(err);
    }
    Ok((output_dir, payload_bytes))
}

struct BoundedArchiveReader<R> {
    inner: R,
    remaining: usize,
}

impl<R: Read> Read for BoundedArchiveReader<R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        if self.remaining == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Recipe archive exceeds its metadata limit",
            ));
        }
        let read_limit = buffer.len().min(self.remaining);
        let read = self.inner.read(&mut buffer[..read_limit])?;
        self.remaining -= read;
        Ok(read)
    }
}

fn extract_bounded_archive(
    reader: impl Read,
    output_dir: &Path,
    max_bytes: usize,
) -> Result<usize> {
    extract_bounded_archive_with_limits(reader, output_dir, max_bytes, ARCHIVE_LIMITS)
}

fn extract_bounded_archive_with_limits(
    reader: impl Read,
    output_dir: &Path,
    max_bytes: usize,
    limits: ArchiveLimits,
) -> Result<usize> {
    let archive_bytes = max_bytes
        .checked_add(limits.overhead_bytes)
        .ok_or_else(|| anyhow!("Recipe archive byte limit is too large"))?;
    let mut archive = Archive::new(BoundedArchiveReader {
        inner: reader,
        remaining: archive_bytes,
    });
    let mut extracted_bytes = 0_usize;
    let mut entry_count = 0;
    let mut path_bytes = 0_usize;
    let mut extracted_paths = HashSet::new();
    for entry in archive.entries()? {
        let mut entry = entry?;
        entry_count += 1;
        if entry_count > limits.entries {
            return Err(anyhow!(
                "Recipe bundle exceeds the {}-entry limit",
                limits.entries
            ));
        }
        let entry_bytes = usize::try_from(entry.size())
            .map_err(|_| anyhow!("Recipe bundle entry size overflowed"))?;
        extracted_bytes = extracted_bytes
            .checked_add(entry_bytes)
            .ok_or_else(|| anyhow!("Recipe bundle size overflowed"))?;
        if extracted_bytes > max_bytes {
            return Err(anyhow!("Recipe bundle exceeds the {max_bytes}-byte limit"));
        }

        let entry_path_bytes = entry.path_bytes().len();
        let link_path_bytes = entry.link_name_bytes().map_or(0, |path| path.len());
        if entry_path_bytes > limits.path_bytes || link_path_bytes > limits.path_bytes {
            return Err(anyhow!(
                "Recipe bundle contains a path exceeding the {}-byte limit",
                limits.path_bytes
            ));
        }
        path_bytes = path_bytes
            .checked_add(entry_path_bytes)
            .and_then(|total| total.checked_add(link_path_bytes))
            .ok_or_else(|| anyhow!("Recipe bundle path size overflowed"))?;
        if path_bytes > limits.total_path_bytes {
            return Err(anyhow!(
                "Recipe bundle exceeds the {}-byte path limit",
                limits.total_path_bytes
            ));
        }

        let mut extracted_path = PathBuf::new();
        for component in entry.path()?.components() {
            extracted_path.push(component);
            extracted_paths.insert(extracted_path.clone());
            if extracted_paths.len() > limits.inodes {
                return Err(anyhow!(
                    "Recipe bundle exceeds the {}-inode limit",
                    limits.inodes
                ));
            }
        }
        if !entry.unpack_in(output_dir)? {
            return Err(anyhow!("Recipe bundle contains an unsafe path"));
        }
    }
    Ok(extracted_bytes)
}

fn list_files(dir: &Path) -> Result<()> {
    println!("{}", style("Files downloaded from github:").bold());
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            println!("  - {}", path.display());
        }
    }
    Ok(())
}

/// Lists all available recipes from a GitHub repository
pub fn list_github_recipes(repo: &str) -> Result<Vec<RecipeInfo>> {
    discover_github_recipes(repo)
}

fn discover_github_recipes(repo: &str) -> Result<Vec<RecipeInfo>> {
    use serde_json::Value;
    use std::process::Command;

    // Ensure GitHub CLI is authenticated
    ensure_gh_authenticated()?;

    // Get repository contents using GitHub CLI
    let output = Command::new("gh")
        .args(["api", &format!("repos/{}/contents", repo)])
        .set_no_window()
        .output()
        .map_err(|e| anyhow!("Failed to fetch repository contents using 'gh api' command (executed when GOOSE_RECIPE_GITHUB_REPO is configured). This requires GitHub CLI (gh) to be installed and authenticated. Error: {}", e))?;

    if !output.status.success() {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("GitHub API request failed: {}", error_msg));
    }

    let contents: Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| anyhow!("Failed to parse GitHub API response: {}", e))?;

    let mut recipes = Vec::new();

    if let Some(items) = contents.as_array() {
        for item in items {
            if let (Some(name), Some(item_type)) = (
                item.get("name").and_then(|n| n.as_str()),
                item.get("type").and_then(|t| t.as_str()),
            ) {
                if item_type == "dir" {
                    // Check if this directory contains a recipe file
                    if let Ok(recipe_info) = check_github_directory_for_recipe(repo, name) {
                        recipes.push(recipe_info);
                    }
                }
            }
        }
    }

    Ok(recipes)
}

fn check_github_directory_for_recipe(repo: &str, dir_name: &str) -> Result<RecipeInfo> {
    use serde_json::Value;
    use std::process::Command;

    // Check directory contents for recipe files
    let output = Command::new("gh")
        .args(["api", &format!("repos/{}/contents/{}", repo, dir_name)])
        .set_no_window()
        .output()
        .map_err(|e| anyhow!("Failed to check directory contents: {}", e))?;

    if !output.status.success() {
        return Err(anyhow!("Failed to access directory: {}", dir_name));
    }

    let contents: Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| anyhow!("Failed to parse directory contents: {}", e))?;

    if let Some(items) = contents.as_array() {
        for item in items {
            if let Some(name) = item.get("name").and_then(|n| n.as_str()) {
                if RECIPE_FILE_EXTENSIONS
                    .iter()
                    .any(|ext| name == format!("recipe.{}", ext))
                {
                    // Found a recipe file, get its content
                    return get_github_recipe_info(repo, dir_name, name);
                }
            }
        }
    }

    Err(anyhow!("No recipe file found in directory: {}", dir_name))
}

fn get_github_recipe_info(repo: &str, dir_name: &str, recipe_filename: &str) -> Result<RecipeInfo> {
    use serde_json::Value;
    use std::process::Command;

    // Get the recipe file content
    let output = Command::new("gh")
        .args([
            "api",
            &format!("repos/{}/contents/{}/{}", repo, dir_name, recipe_filename),
        ])
        .set_no_window()
        .output()
        .map_err(|e| anyhow!("Failed to get recipe file content: {}", e))?;

    if !output.status.success() {
        return Err(anyhow!(
            "Failed to access recipe file: {}/{}",
            dir_name,
            recipe_filename
        ));
    }

    let file_info: Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| anyhow!("Failed to parse file info: {}", e))?;

    if let Some(content_b64) = file_info.get("content").and_then(|c| c.as_str()) {
        // Decode base64 content
        use base64::{engine::general_purpose, Engine as _};
        let content_bytes = general_purpose::STANDARD
            .decode(content_b64.replace('\n', ""))
            .map_err(|e| anyhow!("Failed to decode base64 content: {}", e))?;

        let content = String::from_utf8(content_bytes)
            .map_err(|e| anyhow!("Failed to convert content to string: {}", e))?;

        // Parse the recipe content
        let (recipe, _) = parse_recipe_content(&content, Some(format!("{}/{}", repo, dir_name)))?;

        return Ok(RecipeInfo {
            name: dir_name.to_string(),
            source: RecipeSource::GitHub,
            path: format!("{}/{}", repo, dir_name),
            title: Some(recipe.title),
            description: Some(recipe.description),
        });
    }

    Err(anyhow!("Failed to get recipe content from GitHub"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::io::{Cursor, Read};
    use std::path::Path;
    use std::rc::Rc;

    struct CountingReader {
        inner: Cursor<Vec<u8>>,
        bytes_read: Rc<Cell<usize>>,
    }

    impl Read for CountingReader {
        fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
            let count = self.inner.read(buffer)?;
            self.bytes_read.set(self.bytes_read.get() + count);
            Ok(count)
        }
    }

    fn archive_with_files(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut archive = tar::Builder::new(Vec::new());
        for (path, content) in files {
            let mut header = tar::Header::new_gnu();
            header.set_size(content.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            archive.append_data(&mut header, path, *content).unwrap();
        }
        archive.into_inner().unwrap()
    }

    fn archive_with_extended_metadata(metadata: &[u8]) -> Vec<u8> {
        let mut archive = tar::Builder::new(Vec::new());
        let mut metadata_header = tar::Header::new_gnu();
        metadata_header.set_entry_type(tar::EntryType::XHeader);
        metadata_header.set_size(metadata.len() as u64);
        metadata_header.set_mode(0o644);
        metadata_header.set_cksum();
        archive
            .append_data(&mut metadata_header, "metadata", metadata)
            .unwrap();
        let mut recipe_header = tar::Header::new_gnu();
        recipe_header.set_size(0);
        recipe_header.set_mode(0o644);
        recipe_header.set_cksum();
        archive
            .append_data(&mut recipe_header, "recipe.yaml", &[][..])
            .unwrap();
        archive.into_inner().unwrap()
    }

    #[test]
    fn bounded_archive_rejects_from_header_before_reading_recipe_body() {
        let content = vec![b'x'; 16 * 1024];
        let bytes_read = Rc::new(Cell::new(0));
        let reader = CountingReader {
            inner: Cursor::new(archive_with_files(&[("recipe.yaml", &content)])),
            bytes_read: Rc::clone(&bytes_read),
        };
        let output_dir = tempfile::tempdir().unwrap();
        let output = output_dir.path().join("recipe.yaml");

        let error = extract_bounded_archive(reader, output_dir.path(), content.len() - 1)
            .unwrap_err()
            .to_string();

        assert!(error.contains("byte limit"));
        assert!(bytes_read.get() < content.len());
        assert!(!output.exists());
    }

    #[test]
    fn bounded_archive_accepts_a_recipe_at_the_exact_boundary() {
        let content = b"title: exact boundary";
        let output_dir = tempfile::tempdir().unwrap();

        extract_bounded_archive(
            Cursor::new(archive_with_files(&[("recipe.yaml", content)])),
            output_dir.path(),
            content.len(),
        )
        .unwrap();

        assert_eq!(
            fs::read(output_dir.path().join("recipe.yaml")).unwrap(),
            content
        );
    }

    #[test]
    fn bounded_archive_preserves_sibling_recipe_files() {
        let recipe = b"sub_recipes:\n  - path: '{{ recipe_dir }}/nested/grandchild.yaml'";
        let grandchild = b"title: grandchild";
        let output_dir = tempfile::tempdir().unwrap();

        let payload_bytes = extract_bounded_archive(
            Cursor::new(archive_with_files(&[
                ("recipe.yaml", recipe),
                ("nested/grandchild.yaml", grandchild),
            ])),
            output_dir.path(),
            recipe.len() + grandchild.len(),
        )
        .unwrap();

        assert_eq!(payload_bytes, recipe.len() + grandchild.len());
        assert_eq!(
            fs::read(output_dir.path().join("recipe.yaml")).unwrap(),
            recipe
        );
        assert_eq!(
            fs::read(output_dir.path().join("nested/grandchild.yaml")).unwrap(),
            grandchild
        );
    }

    #[test]
    fn bounded_archive_limits_extended_metadata_reads() {
        let bytes_read = Rc::new(Cell::new(0));
        let reader = CountingReader {
            inner: Cursor::new(archive_with_extended_metadata(&vec![b'x'; 2048])),
            bytes_read: Rc::clone(&bytes_read),
        };
        let output_dir = tempfile::tempdir().unwrap();
        let limits = ArchiveLimits {
            overhead_bytes: 1024,
            ..ARCHIVE_LIMITS
        };

        assert!(extract_bounded_archive_with_limits(reader, output_dir.path(), 0, limits).is_err());
        assert_eq!(bytes_read.get(), limits.overhead_bytes);
        assert!(!output_dir.path().join("recipe.yaml").exists());
    }

    #[test]
    fn bounded_archive_limits_entries_before_creating_another_inode() {
        let output_dir = tempfile::tempdir().unwrap();
        let limits = ArchiveLimits {
            entries: 1,
            ..ARCHIVE_LIMITS
        };

        let error = extract_bounded_archive_with_limits(
            Cursor::new(archive_with_files(&[("first", b""), ("second", b"")])),
            output_dir.path(),
            0,
            limits,
        )
        .unwrap_err()
        .to_string();

        assert!(error.contains("entry limit"));
        assert!(output_dir.path().join("first").exists());
        assert!(!output_dir.path().join("second").exists());
    }

    #[test]
    fn bounded_archive_limits_individual_and_total_path_bytes() {
        let output_dir = tempfile::tempdir().unwrap();
        let individual_path_limits = ArchiveLimits {
            path_bytes: 3,
            ..ARCHIVE_LIMITS
        };
        let total_path_limits = ArchiveLimits {
            total_path_bytes: 5,
            ..ARCHIVE_LIMITS
        };

        let individual_error = extract_bounded_archive_with_limits(
            Cursor::new(archive_with_files(&[("long", b"")])),
            output_dir.path(),
            0,
            individual_path_limits,
        )
        .unwrap_err()
        .to_string();
        let total_error = extract_bounded_archive_with_limits(
            Cursor::new(archive_with_files(&[("one", b""), ("two", b"")])),
            output_dir.path(),
            0,
            total_path_limits,
        )
        .unwrap_err()
        .to_string();

        assert!(individual_error.contains("path exceeding"));
        assert!(total_error.contains("path limit"));
        assert!(!output_dir.path().join("long").exists());
        assert!(output_dir.path().join("one").exists());
        assert!(!output_dir.path().join("two").exists());
    }

    #[test]
    fn bounded_archive_limits_paths_that_would_create_too_many_inodes() {
        let output_dir = tempfile::tempdir().unwrap();
        let limits = ArchiveLimits {
            inodes: 2,
            ..ARCHIVE_LIMITS
        };

        let error = extract_bounded_archive_with_limits(
            Cursor::new(archive_with_files(&[("one/two/recipe.yaml", b"")])),
            output_dir.path(),
            0,
            limits,
        )
        .unwrap_err()
        .to_string();

        assert!(error.contains("inode limit"));
        assert!(!output_dir.path().join("one").exists());
    }

    #[test]
    fn local_repo_path_includes_owner_to_avoid_collisions() {
        let parent = Path::new("goose-recipes");
        let first = get_local_repo_path(parent, "owner-one/shared").unwrap();
        let second = get_local_repo_path(parent, "owner-two/shared").unwrap();

        assert_ne!(first, second);
        assert!(first
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("owner-one__shared-"));
        assert!(second
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("owner-two__shared-"));
    }

    #[test]
    fn local_repo_paths_distinguish_sanitized_name_collisions() {
        let parent = Path::new("goose-recipes");
        let unicode = get_local_repo_path(parent, "owner/café").unwrap();
        let underscore = get_local_repo_path(parent, "owner/caf_").unwrap();

        assert_ne!(unicode, underscore);
        assert!(unicode
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("owner__caf_-"));
        assert!(underscore
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("owner__caf_-"));
    }

    #[test]
    fn temp_child_name_keeps_recipe_downloads_under_temp_dir() {
        let parent = Path::new("goose-recipes");
        let child = temp_child_name("../outside").unwrap();
        let output_dir = parent.join(&child);

        assert_eq!(child, "..__outside");
        assert!(output_dir.starts_with(parent));
    }

    #[test]
    fn temp_child_path_rejects_reserved_components() {
        let parent = Path::new("goose-recipes");

        for name in [
            ".",
            "..",
            "...",
            "....",
            ". ",
            ".. ",
            ". .",
            ".. .",
            "report.",
            "report...",
        ] {
            assert!(temp_child_path(parent, name).is_err(), "accepted {name}");
        }
    }

    #[test]
    fn temp_child_path_preserves_safe_recipe_names() {
        let parent = Path::new("goose-recipes");

        for (name, child) in [
            ("daily-report", "daily-report"),
            ("team/weekly", "team__weekly"),
            ("../outside", "..__outside"),
            ("daily report ", "daily_report_"),
            ("", "_"),
        ] {
            assert_eq!(temp_child_path(parent, name).unwrap(), parent.join(child));
        }
    }

    #[test]
    fn unique_temp_child_paths_include_the_complete_request_identity() {
        let parent = Path::new("goose-recipes");

        assert_ne!(
            unique_temp_child_path(parent, "a/b", "owner/recipes").unwrap(),
            unique_temp_child_path(parent, "a__b", "owner/recipes").unwrap()
        );
        assert_ne!(
            unique_temp_child_path(parent, "daily-report", "owner-one/recipes").unwrap(),
            unique_temp_child_path(parent, "daily-report", "owner-two/recipes").unwrap()
        );
        assert_eq!(
            unique_temp_child_path(parent, "a/b", "owner/recipes").unwrap(),
            unique_temp_child_path(parent, "a/b", "owner/recipes").unwrap()
        );
    }

    #[test]
    fn cleanup_rejects_reserved_components_before_removing_files() {
        let root = tempfile::tempdir().unwrap();
        let parent = root.path().join("recipe-temp");
        fs::create_dir(&parent).unwrap();
        let parent_sentinel = root.path().join("parent-sentinel");
        let child_sentinel = parent.join("child-sentinel");
        fs::write(&parent_sentinel, "keep").unwrap();
        fs::write(&child_sentinel, "keep").unwrap();

        for name in [
            ".",
            "..",
            "...",
            "....",
            ". ",
            ".. ",
            ". .",
            ".. .",
            "report.",
            "report...",
        ] {
            assert!(clean_unique_temp_child_path(&parent, name, "owner/recipes").is_err());
            assert!(parent_sentinel.exists());
            assert!(child_sentinel.exists());
        }
    }

    #[test]
    fn cleanup_removes_only_the_existing_recipe_child() {
        let root = tempfile::tempdir().unwrap();
        let parent = root.path().join("recipe-temp");
        let child = unique_temp_child_path(&parent, "daily-report", "owner/recipes").unwrap();
        fs::create_dir_all(&child).unwrap();
        let parent_sentinel = parent.join("keep");
        fs::write(&parent_sentinel, "keep").unwrap();
        fs::write(child.join("old-recipe.yaml"), "old").unwrap();

        assert_eq!(
            clean_unique_temp_child_path(&parent, "daily-report", "owner/recipes").unwrap(),
            child
        );
        assert!(!child.exists());
        assert!(parent_sentinel.exists());
    }
}
