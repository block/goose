//! Deterministic, application-independent prompt composition and instruction discovery.

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

    fn ignore(&self, root: &Path) -> Gitignore {
        if !self.options.respect_gitignore {
            return Gitignore::empty();
        }
        let mut builder = GitignoreBuilder::new(root);
        let mut dirs: Vec<_> = root.ancestors().collect();
        dirs.reverse();
        for dir in dirs {
            let file = dir.join(".gitignore");
            if file.is_file() {
                builder.add(file);
            }
        }
        builder.build().unwrap_or_else(|_| Gitignore::empty())
    }

    fn allowed(&self, root: &Path, path: &Path, ignore: &Gitignore) -> bool {
        let relative = path.strip_prefix(root).unwrap_or(path).to_string_lossy();
        !relative.split('/').any(|part| part == ".git")
            && !self
                .options
                .exclude_patterns
                .iter()
                .any(|pattern| relative.contains(pattern))
            && (self.options.include_patterns.is_empty()
                || self
                    .options
                    .include_patterns
                    .iter()
                    .any(|pattern| relative.contains(pattern)))
            && !ignore.matched(path, path.is_dir()).is_ignore()
    }

    fn read_directory(&self, root: &Path, directory: &Path) -> Result<Vec<Instruction>> {
        let root = root.canonicalize()?;
        let directory = directory.canonicalize()?;
        if !directory.starts_with(&root) {
            bail!("instruction directory is outside the session root");
        }
        let ignore = self.ignore(&root);
        let mut result = Vec::new();
        for filename in &self.options.filenames {
            let path = directory.join(filename);
            if path.is_file() && self.allowed(&root, &path, &ignore) {
                let canonical = path.canonicalize()?;
                if canonical.starts_with(&root) {
                    result.push(Instruction {
                        key: format!("instructions:{}", canonical.display()),
                        path: canonical,
                        content: std::fs::read_to_string(path)?,
                    });
                }
            }
        }
        Ok(result)
    }

    pub fn discover_root(&self, working_dir: &Path) -> Result<Vec<Instruction>> {
        self.read_directory(working_dir, working_dir)
    }

    pub fn record_tool_arguments(
        &mut self,
        arguments: &Option<Map<String, Value>>,
        working_dir: &Path,
    ) {
        let Some(arguments) = arguments else { return };
        if let Some(path) = arguments.get("path").and_then(Value::as_str) {
            self.record_path(path, working_dir);
        }
        if let Some(command) = arguments.get("command").and_then(Value::as_str) {
            for token in command.split_whitespace().filter(|token| {
                !token.starts_with('-') && (token.contains('/') || token.contains('.'))
            }) {
                self.record_path(token.trim_matches(|c| c == '\'' || c == '"'), working_dir);
            }
        }
    }

    fn record_path(&mut self, path: &str, root: &Path) {
        let path = Path::new(path);
        let joined = if path.is_absolute() {
            path.to_path_buf()
        } else {
            root.join(path)
        };
        let directory = if joined.is_dir() {
            joined
        } else {
            joined.parent().unwrap_or(root).to_path_buf()
        };
        self.pending.push(directory);
    }

    pub fn discover_new_subdirectory_instructions(
        &mut self,
        working_dir: &Path,
    ) -> Result<Vec<Instruction>> {
        let root = working_dir.canonicalize()?;
        let mut pending = std::mem::take(&mut self.pending);
        pending.sort();
        pending.dedup();
        let mut result = Vec::new();
        for directory in pending {
            let Ok(directory) = directory.canonicalize() else {
                continue;
            };
            if directory == root || !directory.starts_with(&root) {
                continue;
            }
            let mut chain: Vec<_> = directory
                .ancestors()
                .take_while(|dir| **dir != root && dir.starts_with(&root))
                .map(Path::to_path_buf)
                .collect();
            chain.reverse();
            for dir in chain {
                if self.loaded.insert(dir.clone()) {
                    result.extend(self.read_directory(&root, &dir)?);
                }
            }
        }
        Ok(result)
    }
}
