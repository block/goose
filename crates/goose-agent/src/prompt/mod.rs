//! Deterministic, application-independent prompt composition and instruction discovery.

pub mod import_files;

use anyhow::{bail, Result};
use goose_provider_types::goose_mode::GooseMode;
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use indexmap::IndexMap;
use minijinja::{Environment, Value as MiniJinjaValue};
use serde::Serialize;
use serde_json::{Map, Value};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use unicode_normalization::UnicodeNormalization;

#[derive(Debug, Clone)]
pub enum PromptSource {
    Template(String),
    Literal(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct PromptContext {
    pub current_date_time: String,
    pub goose_mode: GooseMode,
    #[serde(flatten)]
    pub variables: Map<String, Value>,
}

#[derive(Debug, Clone)]
pub struct PromptComposer {
    base: PromptSource,
    extras: IndexMap<String, String>,
}

fn sanitize(text: &str) -> String {
    text.nfc()
        .filter(|ch| !matches!(ch, '\u{E0000}'..='\u{E007F}'))
        .collect()
}

impl PromptComposer {
    pub fn new(base: PromptSource) -> Self {
        Self {
            base,
            extras: IndexMap::new(),
        }
    }
    pub fn set_base(&mut self, base: PromptSource) {
        self.base = base;
    }
    /// Replacing a key preserves its insertion position.
    pub fn add_extra(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.extras.insert(key.into(), value.into());
    }
    pub fn remove_extra(&mut self, key: &str) {
        self.extras.shift_remove(key);
    }

    pub fn extras(&self) -> &IndexMap<String, String> {
        &self.extras
    }

    /// Renders persistent extras followed by ephemeral round contributions.
    pub fn render(
        &self,
        context: &PromptContext,
        round: impl IntoIterator<Item = (String, String)>,
    ) -> Result<String> {
        let source = match &self.base {
            PromptSource::Template(value) | PromptSource::Literal(value) => sanitize(value),
        };
        let base = match self.base {
            PromptSource::Literal(_) => source,
            PromptSource::Template(_) => {
                let mut env = Environment::new();
                env.set_trim_blocks(true);
                env.set_lstrip_blocks(true);
                env.add_template("system", &source)?;
                env.get_template("system")?
                    .render(MiniJinjaValue::from_serialize(context))?
                    .trim()
                    .to_string()
            }
        };
        let mut extras = self.extras.clone();
        extras.extend(round);
        if extras.is_empty() {
            return Ok(base);
        }
        Ok(format!(
            "{base}\n\n# Additional Instructions:\n\n{}",
            extras
                .into_values()
                .map(|value| sanitize(&value))
                .collect::<Vec<_>>()
                .join("\n\n")
        ))
    }
}

#[derive(Debug, Clone)]
pub struct InstructionDiscoveryOptions {
    pub filenames: Vec<String>,
    pub respect_gitignore: bool,
    pub include_patterns: Vec<String>,
    pub exclude_patterns: Vec<String>,
}

impl Default for InstructionDiscoveryOptions {
    fn default() -> Self {
        Self {
            filenames: vec![".goosehints".into(), "AGENTS.md".into()],
            respect_gitignore: true,
            include_patterns: Vec::new(),
            exclude_patterns: vec![".git".into()],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instruction {
    pub key: String,
    pub path: PathBuf,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct InstructionDiscovery {
    options: InstructionDiscoveryOptions,
    pending: Vec<PathBuf>,
    loaded: HashSet<PathBuf>,
}

impl InstructionDiscovery {
    pub fn new(options: InstructionDiscoveryOptions) -> Self {
        Self {
            options,
            pending: Vec::new(),
            loaded: HashSet::new(),
        }
    }

    pub fn discover_root(&self, working_dir: &Path) -> Result<Vec<Instruction>> {
        let directories = match find_git_root(working_dir) {
            Some(root) => {
                let mut directories: Vec<_> = working_dir
                    .ancestors()
                    .take_while(|directory| directory.starts_with(root))
                    .map(Path::to_path_buf)
                    .collect();
                directories.reverse();
                directories
            }
            None => vec![working_dir.to_path_buf()],
        };
        let boundary = find_git_root(working_dir).unwrap_or(working_dir);
        self.discover_directories(boundary, &directories, "instructions")
    }

    pub fn record_tool_arguments(
        &mut self,
        arguments: &Option<Map<String, Value>>,
        working_dir: &Path,
    ) {
        let Some(arguments) = arguments else { return };
        if let Some(path) = arguments.get("path").and_then(Value::as_str) {
            if let Some(directory) = resolve_parent(path, working_dir) {
                self.pending.push(directory);
            }
        }
        if let Some(command) = arguments.get("command").and_then(Value::as_str) {
            for token in shell_words::split(command).unwrap_or_default() {
                if !token.starts_with('-')
                    && (token.contains(std::path::MAIN_SEPARATOR) || token.contains('.'))
                {
                    if let Some(directory) = resolve_parent(&token, working_dir) {
                        self.pending.push(directory);
                    }
                }
            }
        }
    }

    pub fn discover_new_subdirectory_instructions(
        &mut self,
        working_dir: &Path,
    ) -> Result<Vec<Instruction>> {
        let root = working_dir.canonicalize()?;
        let pending = std::mem::take(&mut self.pending);
        let mut result = Vec::new();
        for directory in pending {
            let Ok(directory) = directory.canonicalize() else {
                continue;
            };
            if directory == root
                || !directory.starts_with(&root)
                || !self.loaded.insert(directory.clone())
            {
                continue;
            }
            let mut chain: Vec<_> = directory
                .ancestors()
                .take_while(|dir| **dir != root && dir.starts_with(&root))
                .map(Path::to_path_buf)
                .collect();
            chain.reverse();
            let mut discovered = self.discover_directories(&root, &chain, "subdir_hints")?;
            if !discovered.is_empty() {
                let content = discovered
                    .drain(..)
                    .map(|instruction| instruction.content)
                    .collect::<Vec<_>>()
                    .join("\n");
                result.push(Instruction {
                    key: format!("subdir_hints:{}", directory.display()),
                    path: directory.clone(),
                    content: format!(
                        "### Subdirectory Hints ({})\n{}",
                        directory.display(),
                        content
                    ),
                });
            }
        }
        Ok(result)
    }

    fn discover_directories(
        &self,
        working_dir: &Path,
        directories: &[PathBuf],
        prefix: &str,
    ) -> Result<Vec<Instruction>> {
        let root = working_dir.canonicalize()?;
        let import_boundary = find_git_root(&root).unwrap_or(&root);
        let ignore = if self.options.respect_gitignore {
            build_gitignore_with_root(
                &root,
                directories.last().map(PathBuf::as_path).unwrap_or(&root),
            )
        } else {
            Gitignore::empty()
        };
        let mut result = Vec::new();
        for directory in directories {
            let canonical_directory = directory.canonicalize()?;
            if !canonical_directory.starts_with(&root) {
                bail!("instruction directory is outside the session root");
            }
            for filename in &self.options.filenames {
                let path = canonical_directory.join(filename);
                if !path.is_file() || ignored(&root, &path, &ignore, &self.options) {
                    continue;
                }
                let canonical = path.canonicalize()?;
                if !canonical.starts_with(import_boundary) {
                    continue;
                }
                let mut visited = HashSet::new();
                let content = import_files::read_referenced_files(
                    &path,
                    import_boundary,
                    &mut visited,
                    0,
                    &ignore,
                );
                if !content.is_empty() {
                    result.push(Instruction {
                        key: format!("{prefix}:{}", canonical.display()),
                        path: canonical,
                        content,
                    });
                }
            }
        }
        Ok(result)
    }
}

fn resolve_parent(value: &str, working_dir: &Path) -> Option<PathBuf> {
    let path = Path::new(value);
    let resolved = if path.is_absolute() {
        path.to_path_buf()
    } else {
        working_dir.join(path)
    };
    resolved.parent().map(Path::to_path_buf)
}

pub fn find_git_root(start: &Path) -> Option<&Path> {
    start
        .ancestors()
        .find(|directory| directory.join(".git").exists())
}

pub fn build_gitignore(cwd: &Path) -> Gitignore {
    build_gitignore_with_root(find_git_root(cwd).unwrap_or(cwd), cwd)
}

fn build_gitignore_with_root(root: &Path, cwd: &Path) -> Gitignore {
    let mut directories: Vec<_> = cwd
        .ancestors()
        .take_while(|dir| dir.starts_with(root))
        .collect();
    directories.reverse();
    let mut builder = GitignoreBuilder::new(root);
    for directory in directories {
        let path = directory.join(".gitignore");
        if path.is_file() {
            builder.add(path);
        }
    }
    builder.build().unwrap_or_else(|_| Gitignore::empty())
}

fn ignored(
    root: &Path,
    path: &Path,
    ignore: &Gitignore,
    options: &InstructionDiscoveryOptions,
) -> bool {
    let relative = path.strip_prefix(root).unwrap_or(path).to_string_lossy();
    relative.split('/').any(|part| part == ".git")
        || options
            .exclude_patterns
            .iter()
            .any(|pattern| relative.contains(pattern))
        || (!options.include_patterns.is_empty()
            && !options
                .include_patterns
                .iter()
                .any(|pattern| relative.contains(pattern)))
        || path
            .ancestors()
            .take_while(|ancestor| ancestor.starts_with(root))
            .any(|ancestor| ignore.matched(ancestor, ancestor.is_dir()).is_ignore())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;
    use tempfile::TempDir;

    fn options() -> InstructionDiscoveryOptions {
        InstructionDiscoveryOptions {
            filenames: vec!["AGENTS.md".into()],
            ..Default::default()
        }
    }

    fn arguments(key: &str, value: &str) -> Option<Map<String, Value>> {
        Some(serde_json::from_value(json!({ key: value })).unwrap())
    }

    #[test]
    fn discovers_root_and_parent_instructions_in_order() {
        let root = TempDir::new().unwrap();
        fs::create_dir(root.path().join(".git")).unwrap();
        fs::write(root.path().join("AGENTS.md"), "root").unwrap();
        let nested = root.path().join("one/two");
        fs::create_dir_all(&nested).unwrap();
        fs::write(root.path().join("one/AGENTS.md"), "one").unwrap();
        fs::write(nested.join("AGENTS.md"), "two").unwrap();

        let found = InstructionDiscovery::new(options())
            .discover_root(&nested)
            .unwrap();
        assert_eq!(
            found
                .into_iter()
                .map(|item| item.content)
                .collect::<Vec<_>>(),
            ["root", "one", "two"]
        );
    }

    #[test]
    fn tracks_path_and_command_arguments_and_deduplicates() {
        let root = TempDir::new().unwrap();
        let nested = root.path().join("src/deep");
        fs::create_dir_all(&nested).unwrap();
        fs::write(root.path().join("src/AGENTS.md"), "src").unwrap();
        fs::write(nested.join("AGENTS.md"), "deep").unwrap();
        let mut discovery = InstructionDiscovery::new(options());
        discovery.record_tool_arguments(&arguments("path", "src/deep/file.rs"), root.path());
        discovery.record_tool_arguments(&arguments("command", "cat src/deep/file.rs"), root.path());
        let found = discovery
            .discover_new_subdirectory_instructions(root.path())
            .unwrap();
        assert_eq!(found.len(), 1);
        assert!(found[0].content.contains("src"));
        assert!(found[0].content.contains("deep"));
        discovery.record_tool_arguments(&arguments("path", "src/deep/other.rs"), root.path());
        assert!(discovery
            .discover_new_subdirectory_instructions(root.path())
            .unwrap()
            .is_empty());
    }

    #[test]
    fn sibling_transitions_load_each_directory_once() {
        let root = TempDir::new().unwrap();
        for name in ["left", "right"] {
            fs::create_dir(root.path().join(name)).unwrap();
            fs::write(root.path().join(name).join("AGENTS.md"), name).unwrap();
        }
        let mut discovery = InstructionDiscovery::new(options());
        discovery.record_tool_arguments(&arguments("path", "left/file"), root.path());
        assert!(discovery
            .discover_new_subdirectory_instructions(root.path())
            .unwrap()[0]
            .content
            .contains("left"));
        discovery.record_tool_arguments(&arguments("path", "right/file"), root.path());
        assert!(discovery
            .discover_new_subdirectory_instructions(root.path())
            .unwrap()[0]
            .content
            .contains("right"));
    }

    #[test]
    fn retries_after_missing_directory_is_created() {
        let root = TempDir::new().unwrap();
        let mut discovery = InstructionDiscovery::new(options());
        discovery.record_tool_arguments(&arguments("path", "later/file"), root.path());
        assert!(discovery
            .discover_new_subdirectory_instructions(root.path())
            .unwrap()
            .is_empty());
        fs::create_dir(root.path().join("later")).unwrap();
        fs::write(root.path().join("later/AGENTS.md"), "later").unwrap();
        discovery.record_tool_arguments(&arguments("path", "later/file"), root.path());
        assert_eq!(
            discovery
                .discover_new_subdirectory_instructions(root.path())
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn rejects_traversal_and_escaping_symlinks() {
        let root = TempDir::new().unwrap();
        let outside = TempDir::new().unwrap();
        fs::write(outside.path().join("AGENTS.md"), "secret").unwrap();
        let mut discovery = InstructionDiscovery::new(options());
        discovery.record_tool_arguments(&arguments("path", "../outside/file"), root.path());
        assert!(discovery
            .discover_new_subdirectory_instructions(root.path())
            .unwrap()
            .is_empty());
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(outside.path(), root.path().join("escape")).unwrap();
            discovery.record_tool_arguments(&arguments("path", "escape/file"), root.path());
            assert!(discovery
                .discover_new_subdirectory_instructions(root.path())
                .unwrap()
                .is_empty());
        }
    }

    #[test]
    fn respects_gitignore_for_instruction_files() {
        let root = TempDir::new().unwrap();
        fs::create_dir(root.path().join("ignored")).unwrap();
        fs::write(root.path().join(".gitignore"), "ignored/\n").unwrap();
        fs::write(root.path().join("ignored/AGENTS.md"), "secret").unwrap();
        let mut discovery = InstructionDiscovery::new(options());
        discovery.record_tool_arguments(&arguments("path", "ignored/file"), root.path());
        assert!(discovery
            .discover_new_subdirectory_instructions(root.path())
            .unwrap()
            .is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn in_boundary_symlink_alias_is_loaded_once() {
        let root = TempDir::new().unwrap();
        fs::create_dir(root.path().join("real")).unwrap();
        fs::write(root.path().join("real/AGENTS.md"), "real").unwrap();
        std::os::unix::fs::symlink(root.path().join("real"), root.path().join("alias")).unwrap();
        let mut discovery = InstructionDiscovery::new(options());
        discovery.record_tool_arguments(&arguments("path", "alias/file"), root.path());
        assert_eq!(
            discovery
                .discover_new_subdirectory_instructions(root.path())
                .unwrap()
                .len(),
            1
        );
        discovery.record_tool_arguments(&arguments("path", "real/file"), root.path());
        assert!(discovery
            .discover_new_subdirectory_instructions(root.path())
            .unwrap()
            .is_empty());
    }
}
