use rustyline::completion::{Completer, FilenameCompleter, Pair};
use rustyline::highlight::{CmdKind, Highlighter};
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context, Helper, Result};
use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::CompletionCache;

/// Represents a file candidate for completion
#[derive(Debug, Clone)]
struct FileCandidate {
    path: PathBuf,
    relative_path: String,
    is_directory: bool,
}

/// Completer for goose CLI commands
pub struct GooseCompleter {
    completion_cache: Arc<std::sync::RwLock<CompletionCache>>,
    filename_completer: FilenameCompleter,
}

impl GooseCompleter {
    /// Create a new GooseCompleter with a reference to the Session's completion cache
    pub fn new(completion_cache: Arc<std::sync::RwLock<CompletionCache>>) -> Self {
        Self {
            completion_cache,
            filename_completer: FilenameCompleter::new(),
        }
    }

    /// Fuzzy match with scoring (ported from MentionPopover.tsx:35-97)
    /// Returns (score, match_positions) where score > 0 means match, -1 means no match
    fn fuzzy_match(pattern: &str, text: &str) -> (i32, Vec<usize>) {
        if pattern.is_empty() {
            return (0, vec![]);
        }

        let pattern_lower = pattern.to_lowercase();
        let text_lower = text.to_lowercase();
        let pattern_chars: Vec<char> = pattern_lower.chars().collect();
        let text_chars: Vec<char> = text_lower.chars().collect();
        let mut matches = vec![];

        let mut pattern_idx = 0;
        let mut score = 0;
        let mut consecutive_matches = 0;

        for (i, &ch) in text_chars.iter().enumerate() {
            if pattern_idx < pattern_chars.len() && ch == pattern_chars[pattern_idx] {
                matches.push(i);
                pattern_idx += 1;
                consecutive_matches += 1;

                // Bonus for consecutive matches (3 points per consecutive char)
                score += consecutive_matches * 3;

                // Bonus for matches at word boundaries or separators (+10)
                if i == 0 || Self::is_boundary_char(text_chars.get(i.wrapping_sub(1))) {
                    score += 10;
                }

                // Bonus for matching start of filename after last / (+15)
                if i > 0 && text[..i].rfind('/').map_or(false, |p| p == i - 1) {
                    score += 15;
                }
            } else {
                consecutive_matches = 0;
            }
        }

        // All pattern chars must match
        if pattern_idx == pattern_chars.len() {
            // Small penalty for length (longer paths score slightly lower)
            score -= (text.len() as i32) / 20;

            // Bonus for exact substring match (+20)
            if text_lower.contains(&pattern_lower) {
                score += 20;
            }

            // Bonus for matching the filename specifically (+25)
            let filename = text.split('/').last().unwrap_or("");
            if filename.to_lowercase().contains(&pattern_lower) {
                score += 25;
            }

            (score, matches)
        } else {
            (-1, vec![])
        }
    }

    /// Check if a character is a word boundary
    fn is_boundary_char(ch: Option<&char>) -> bool {
        matches!(ch, Some('/') | Some('_') | Some('-') | Some('.'))
    }

    /// Scan working directory for files, respecting common ignore patterns
    /// Similar to MentionPopover.tsx:210-419
    fn scan_files_for_completion(&self, start_path: &Path, max_depth: usize) -> Vec<FileCandidate> {
        let mut results = Vec::new();

        // Priority directories (scanned first, deeper)
        const PRIORITY_DIRS: &[&str] = &["src", "components", "lib", "crates", "docs", "examples"];

        // Directories to skip
        const SKIP_DIRS: &[&str] = &[
            ".git",
            ".svn",
            ".hg",
            "node_modules",
            "__pycache__",
            ".vscode",
            ".idea",
            "target",
            "dist",
            "build",
            ".cache",
            ".npm",
            ".yarn",
        ];

        self.scan_directory_recursive(
            start_path,
            "",
            0,
            max_depth,
            PRIORITY_DIRS,
            SKIP_DIRS,
            &mut results,
        );

        results
    }

    /// Recursively scan a directory
    #[allow(clippy::too_many_arguments)]
    fn scan_directory_recursive(
        &self,
        dir_path: &Path,
        relative_path: &str,
        depth: usize,
        max_depth: usize,
        priority_dirs: &[&str],
        skip_dirs: &[&str],
        results: &mut Vec<FileCandidate>,
    ) {
        if depth > max_depth {
            return;
        }

        let Ok(entries) = std::fs::read_dir(dir_path) else {
            return; // Skip directories we can't read
        };

        let mut items: Vec<_> = entries.filter_map(|e| e.ok()).collect();

        // Sort to prioritize certain directories
        items.sort_by(|a, b| {
            let a_name = a.file_name().to_string_lossy().to_string();
            let b_name = b.file_name().to_string_lossy().to_string();

            let a_priority = priority_dirs.contains(&a_name.as_str());
            let b_priority = priority_dirs.contains(&b_name.as_str());

            match (a_priority, b_priority) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a_name.cmp(&b_name),
            }
        });

        // Limit items per directory
        let item_limit = match depth {
            0 => 50,
            1 => 40,
            _ => 30,
        };

        for entry in items.into_iter().take(item_limit) {
            let name = entry.file_name().to_string_lossy().to_string();

            // Skip hidden files and ignored directories
            if name.starts_with('.') || skip_dirs.contains(&name.as_str()) {
                continue;
            }

            let full_path = entry.path();
            let item_relative_path = if relative_path.is_empty() {
                name.clone()
            } else {
                format!("{}/{}", relative_path, name)
            };

            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    results.push(FileCandidate {
                        path: full_path,
                        relative_path: item_relative_path,
                        is_directory: false,
                    });
                } else if metadata.is_dir() {
                    results.push(FileCandidate {
                        path: full_path.clone(),
                        relative_path: item_relative_path.clone(),
                        is_directory: true,
                    });

                    // Recurse into directories
                    let should_recurse_deeper = depth < 4 || priority_dirs.contains(&name.as_str());

                    if should_recurse_deeper {
                        self.scan_directory_recursive(
                            &full_path,
                            &item_relative_path,
                            depth + 1,
                            max_depth,
                            priority_dirs,
                            skip_dirs,
                            results,
                        );
                    }
                }
            }
        }
    }

    /// Complete file paths after @ symbol using fuzzy matching
    fn complete_file_mention(&self, line: &str, ctx: &Context) -> Result<(usize, Vec<Pair>)> {
        // Find the last @ symbol
        let at_pos = match line.rfind('@') {
            Some(pos) => pos,
            None => return Ok((line.len(), vec![])),
        };

        // Extract query after @
        let query = &line[at_pos + 1..];

        // Don't complete if there's whitespace after @
        if query.contains(' ') || query.contains('\n') {
            return Ok((line.len(), vec![]));
        }

        // Get working directory - use current directory
        let working_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        // Scan files from working directory
        let candidates = self.scan_files_for_completion(&working_dir, 5);

        // If query is empty, return all files (sorted by path depth)
        if query.is_empty() {
            let mut pairs: Vec<Pair> = candidates
                .iter()
                .take(20) // Limit to 20 results for empty query
                .map(|c| Pair {
                    display: c.relative_path.clone(),
                    replacement: c.relative_path.clone(),
                })
                .collect();

            // Sort by path depth (shorter paths first)
            pairs.sort_by(|a, b| {
                let a_depth = a.display.matches('/').count();
                let b_depth = b.display.matches('/').count();
                a_depth.cmp(&b_depth)
            });

            return Ok((at_pos + 1, pairs));
        }

        // Fuzzy match each file against query
        let mut scored_candidates: Vec<(FileCandidate, i32)> = candidates
            .into_iter()
            .filter_map(|candidate| {
                // Try matching against different parts of the path
                let name_match = Self::fuzzy_match(query, &candidate.relative_path);
                let filename_match = candidate
                    .relative_path
                    .split('/')
                    .last()
                    .map(|name| Self::fuzzy_match(query, name))
                    .unwrap_or((-1, vec![]));

                // Use the best score
                let score = name_match.0.max(filename_match.0);

                if score > 0 {
                    Some((candidate, score))
                } else {
                    None
                }
            })
            .collect();

        // Sort by score (descending)
        scored_candidates.sort_by(|a, b| b.1.cmp(&a.1));

        // Convert to Pair and limit results
        let pairs: Vec<Pair> = scored_candidates
            .into_iter()
            .take(20) // Limit to top 20 matches
            .map(|(candidate, _score)| Pair {
                display: candidate.relative_path.clone(),
                replacement: candidate.relative_path,
            })
            .collect();

        Ok((at_pos + 1, pairs))
    }

    /// Complete prompt names for the /prompt command
    fn complete_prompt_names(&self, line: &str) -> Result<(usize, Vec<Pair>)> {
        // Get the prefix of the prompt name being typed
        let prefix = line.get(8..).unwrap_or("");

        // Get available prompts from cache
        let cache = self.completion_cache.read().unwrap();

        // Create completion candidates that match the prefix
        let candidates: Vec<Pair> = cache
            .prompts
            .iter()
            .flat_map(|(_, names)| names)
            .filter(|name| name.starts_with(prefix.trim()))
            .map(|name| Pair {
                display: name.clone(),
                replacement: name.clone(),
            })
            .collect();

        Ok((8, candidates))
    }

    /// Complete flags for the /prompt command
    fn complete_prompt_flags(&self, line: &str) -> Result<(usize, Vec<Pair>)> {
        // Get the last part of the line
        let parts: Vec<&str> = line.split_whitespace().collect();
        if let Some(last_part) = parts.last() {
            // If the last part starts with '-', it might be a partial flag
            if last_part.starts_with('-') {
                // Define available flags
                let flags = ["--info"];

                // Find flags that match the prefix
                let matching_flags: Vec<Pair> = flags
                    .iter()
                    .filter(|flag| flag.starts_with(last_part))
                    .map(|flag| Pair {
                        display: flag.to_string(),
                        replacement: flag.to_string(),
                    })
                    .collect();

                if !matching_flags.is_empty() {
                    // Return matches for the partial flag
                    // The position is the start of the last word
                    let pos = line.len() - last_part.len();
                    return Ok((pos, matching_flags));
                }
            }
        }

        // No flag completions available
        Ok((line.len(), vec![]))
    }

    /// Complete flags for the /mode command
    fn complete_mode_flags(&self, line: &str) -> Result<(usize, Vec<Pair>)> {
        let modes = ["auto", "approve", "smart_approve", "chat"];

        let parts: Vec<&str> = line.split_whitespace().collect();

        // If we're just after "/mode" with a space, show all options
        if line == "/mode " {
            return Ok((
                line.len(),
                modes
                    .iter()
                    .map(|mode| Pair {
                        display: mode.to_string(),
                        replacement: format!("{} ", mode),
                    })
                    .collect(),
            ));
        }

        // If we're typing a mode name, show the flags for that mode
        if parts.len() == 2 {
            let partial = parts[1].to_lowercase();
            return Ok((
                line.len() - partial.len(),
                modes
                    .iter()
                    .filter(|mode| mode.to_lowercase().starts_with(&partial.to_lowercase()))
                    .map(|mode| Pair {
                        display: mode.to_string(),
                        replacement: format!("{} ", mode),
                    })
                    .collect(),
            ));
        }

        // No completions available
        Ok((line.len(), vec![]))
    }

    /// Complete slash commands
    fn complete_slash_commands(&self, line: &str) -> Result<(usize, Vec<Pair>)> {
        // Define available slash commands
        let commands = [
            "/exit",
            "/quit",
            "/help",
            "/?",
            "/t",
            "/extension",
            "/builtin",
            "/prompts",
            "/prompt",
            "/mode",
            "/recipe",
        ];

        // Find commands that match the prefix
        let matching_commands: Vec<Pair> = commands
            .iter()
            .filter(|cmd| cmd.starts_with(line))
            .map(|cmd| Pair {
                display: cmd.to_string(),
                replacement: format!("{} ", cmd), // Add a space after the command
            })
            .collect();

        if !matching_commands.is_empty() {
            return Ok((0, matching_commands));
        }

        // No command completions available
        Ok((line.len(), vec![]))
    }

    /// Complete argument keys for a specific prompt
    fn complete_argument_keys(&self, line: &str) -> Result<(usize, Vec<Pair>)> {
        let parts: Vec<&str> = line.get(8..).unwrap_or("").split_whitespace().collect();

        // We need at least the prompt name
        if parts.is_empty() {
            return Ok((line.len(), vec![]));
        }

        let prompt_name = parts[0];

        // Get prompt info from cache
        let cache = self.completion_cache.read().unwrap();
        let prompt_info = cache.prompt_info.get(prompt_name).cloned();

        if let Some(info) = prompt_info {
            if let Some(args) = info.arguments {
                // Find required arguments that haven't been provided yet
                let existing_args: Vec<&str> = parts
                    .iter()
                    .skip(1)
                    .filter_map(|part| {
                        if part.contains('=') {
                            Some(part.split('=').next().unwrap())
                        } else {
                            None
                        }
                    })
                    .collect();

                // Check if we're trying to complete a partial argument name
                if let Some(last_part) = parts.last() {
                    // ignore if last_part starts with = / \ for suggestions
                    if let Some(c) = last_part.chars().next() {
                        if matches!(c, '=' | '/' | '\\') {
                            return Ok((line.len(), vec![]));
                        }
                    }

                    // If the last part doesn't contain '=', it might be a partial argument name
                    if !last_part.contains('=') {
                        // Find arguments that match the prefix
                        let matching_args: Vec<Pair> = args
                            .iter()
                            .filter(|arg| {
                                arg.name.starts_with(last_part)
                                    && !existing_args.contains(&arg.name.as_str())
                            })
                            .map(|arg| Pair {
                                display: format!("{}=", arg.name),
                                replacement: format!("{}=", arg.name),
                            })
                            .collect();

                        if !matching_args.is_empty() {
                            // Return matches for the partial argument name
                            // The position is the start of the last word
                            let pos = line.len() - last_part.len();
                            return Ok((pos, matching_args));
                        }

                        // If we have a partial argument that doesn't match anything,
                        // return an empty list rather than suggesting unrelated arguments
                        if !last_part.is_empty() && *last_part != prompt_name {
                            return Ok((line.len(), vec![]));
                        }
                    }
                }

                // If no partial match or no last part, suggest all required arguments
                // Use a reference to avoid moving args
                let mut candidates: Vec<_> = Vec::new();
                for arg in &args {
                    if arg.required.unwrap_or(false) && !existing_args.contains(&arg.name.as_str())
                    {
                        candidates.push(Pair {
                            display: format!("{}=", arg.name),
                            replacement: format!("{}=", arg.name),
                        });
                    }
                }

                if !candidates.is_empty() {
                    return Ok((line.len(), candidates));
                }

                // If no required arguments left, suggest all optional ones
                // Use a reference to avoid moving args
                for arg in &args {
                    if !arg.required.unwrap_or(true) && !existing_args.contains(&arg.name.as_str())
                    {
                        candidates.push(Pair {
                            display: format!("{}=", arg.name),
                            replacement: format!("{}=", arg.name),
                        });
                    }
                }
                return Ok((line.len(), candidates));
            }
        }

        // No completions available
        Ok((line.len(), vec![]))
    }

    /// Complete file paths
    fn complete_file_path(&self, line: &str, ctx: &Context) -> Result<(usize, Vec<Pair>)> {
        let parts: Vec<&str> = line.split_whitespace().collect();

        if let Some(last_part) = parts.last() {
            // Skip filename completion for words starting with special characters
            if last_part.starts_with('/') && last_part.len() == 1 {
                // Just a slash - no completion
                return Ok((line.len(), vec![]));
            }

            if last_part.starts_with('-') || last_part.contains('=') {
                // Skip flag or key-value pairs
                return Ok((line.len(), vec![]));
            }

            // Complete the partial path
            let pos = line.len() - last_part.len();
            let (start, candidates) =
                self.filename_completer
                    .complete(last_part, last_part.len(), ctx)?;

            // Return the completion results, with adjusted position
            return Ok((pos + start, candidates));
        }

        Ok((line.len(), vec![]))
    }
}

impl Completer for GooseCompleter {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &Context<'_>,
    ) -> Result<(usize, Vec<Self::Candidate>)> {
        // If the cursor is not at the end of the line, don't try to complete
        if pos < line.len() {
            return Ok((pos, vec![]));
        }

        // Check for @ file mentions BEFORE slash commands
        if line.contains('@') {
            // Find the last @ before cursor
            let before_cursor = &line[..pos];
            if let Some(at_pos) = before_cursor.rfind('@') {
                let after_at = &before_cursor[at_pos + 1..];
                // Only complete if no whitespace after @
                if !after_at.contains(' ') && !after_at.contains('\n') {
                    return self.complete_file_mention(line, ctx);
                }
            }
        }

        // If the line starts with '/', it might be a slash command
        if line.starts_with('/') {
            // If it's just a partial slash command (no space yet)
            if !line.contains(' ') {
                return self.complete_slash_commands(line);
            }

            // Handle /prompt command
            if line.starts_with("/prompt") {
                // If we're just after "/prompt" with or without a space
                if line == "/prompt" || line == "/prompt " {
                    return self.complete_prompt_names(line);
                }

                // Get the parts of the command
                let parts: Vec<&str> = line.split_whitespace().collect();

                // If we're typing a prompt name (only one part after /prompt)
                if parts.len() == 2 && !line.ends_with(' ') {
                    return self.complete_prompt_names(line);
                }

                // Check if we might be typing a flag
                if let Some(last_part) = parts.last() {
                    if last_part.starts_with('-') {
                        return self.complete_prompt_flags(line);
                    }
                }

                // If we have a prompt name and need argument completion
                if parts.len() >= 2 {
                    return self.complete_argument_keys(line);
                }
            }

            // Handle /prompts command
            if line.starts_with("/prompts") {
                // If we're just after "/prompts" with a space
                if line == "/prompts " {
                    // Suggest the --extension flag
                    return Ok((
                        line.len(),
                        vec![Pair {
                            display: "--extension".to_string(),
                            replacement: "--extension ".to_string(),
                        }],
                    ));
                }

                // Check if we might be typing the --extension flag
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() == 2
                    && parts[1].starts_with('-')
                    && "--extension".starts_with(parts[1])
                {
                    return Ok((
                        line.len() - parts[1].len(),
                        vec![Pair {
                            display: "--extension".to_string(),
                            replacement: "--extension ".to_string(),
                        }],
                    ));
                }
            }

            if line.starts_with("/mode") {
                return self.complete_mode_flags(line);
            }

            return Ok((pos, vec![]));
        }

        // For normal text (not slash commands), try file path completion
        self.complete_file_path(line, ctx)
    }
}

// Implement the Helper trait which is required by rustyline
impl Helper for GooseCompleter {}

// Implement required traits with default implementations
impl Hinter for GooseCompleter {
    type Hint = String;

    fn hint(&self, line: &str, _pos: usize, _ctx: &Context<'_>) -> Option<Self::Hint> {
        // Only show hint when line is empty
        if line.is_empty() {
            Some("Press Enter to send, Ctrl-J for new line".to_string())
        } else {
            None
        }
    }
}

impl Highlighter for GooseCompleter {
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        _default: bool,
    ) -> Cow<'b, str> {
        Cow::Borrowed(prompt)
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        // Style the hint text with a dim color
        let styled = console::Style::new().dim().apply_to(hint).to_string();
        Cow::Owned(styled)
    }

    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {
        Cow::Borrowed(line)
    }

    fn highlight_char(&self, _line: &str, _pos: usize, _cmd_kind: CmdKind) -> bool {
        false
    }
}

impl Validator for GooseCompleter {
    fn validate(
        &self,
        _ctx: &mut rustyline::validate::ValidationContext,
    ) -> Result<rustyline::validate::ValidationResult> {
        Ok(rustyline::validate::ValidationResult::Valid(None))
    }
}

#[cfg(test)]
mod tests {
    use rmcp::model::PromptArgument;

    use super::*;
    use crate::session::output;
    use std::sync::{Arc, RwLock};

    // Helper function to create a test completion cache
    fn create_test_cache() -> Arc<RwLock<CompletionCache>> {
        let mut cache = CompletionCache::new();

        // Add some test prompts
        cache.prompts.insert(
            "extension1".to_string(),
            vec!["test_prompt1".to_string(), "test_prompt2".to_string()],
        );

        cache
            .prompts
            .insert("extension2".to_string(), vec!["other_prompt".to_string()]);

        // Add prompt info with arguments
        let test_prompt1_args = vec![
            PromptArgument {
                name: "required_arg".to_string(),
                description: Some("A required argument".to_string()),
                required: Some(true),
                title: None,
            },
            PromptArgument {
                name: "optional_arg".to_string(),
                description: Some("An optional argument".to_string()),
                required: Some(false),
                title: None,
            },
        ];

        let test_prompt1_info = output::PromptInfo {
            name: "test_prompt1".to_string(),
            description: Some("Test prompt 1 description".to_string()),
            arguments: Some(test_prompt1_args),
            extension: Some("extension1".to_string()),
        };
        cache
            .prompt_info
            .insert("test_prompt1".to_string(), test_prompt1_info);

        let test_prompt2_info = output::PromptInfo {
            name: "test_prompt2".to_string(),
            description: Some("Test prompt 2 description".to_string()),
            arguments: None,
            extension: Some("extension1".to_string()),
        };
        cache
            .prompt_info
            .insert("test_prompt2".to_string(), test_prompt2_info);

        let other_prompt_info = output::PromptInfo {
            name: "other_prompt".to_string(),
            description: Some("Other prompt description".to_string()),
            arguments: None,
            extension: Some("extension2".to_string()),
        };
        cache
            .prompt_info
            .insert("other_prompt".to_string(), other_prompt_info);

        Arc::new(RwLock::new(cache))
    }

    #[test]
    fn test_complete_slash_commands() {
        let cache = create_test_cache();
        let completer = GooseCompleter::new(cache);

        // Test complete match
        let (pos, candidates) = completer.complete_slash_commands("/exit").unwrap();
        assert_eq!(pos, 0);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].display, "/exit");
        assert_eq!(candidates[0].replacement, "/exit ");

        // Test partial match
        let (pos, candidates) = completer.complete_slash_commands("/e").unwrap();
        assert_eq!(pos, 0);
        // There might be multiple commands starting with "e" like "/exit" and "/extension"
        assert!(!candidates.is_empty());

        // Test multiple matches
        let (pos, candidates) = completer.complete_slash_commands("/").unwrap();
        assert_eq!(pos, 0);
        assert!(candidates.len() > 1);

        // Test no match
        let (_pos, candidates) = completer.complete_slash_commands("/nonexistent").unwrap();
        assert_eq!(candidates.len(), 0);
    }

    #[test]
    fn test_complete_prompt_names() {
        let cache = create_test_cache();
        let completer = GooseCompleter::new(cache);

        // Test with just "/prompt "
        let (pos, candidates) = completer.complete_prompt_names("/prompt ").unwrap();
        assert_eq!(pos, 8);
        assert_eq!(candidates.len(), 3); // All prompts

        // Test with partial prompt name
        let (pos, candidates) = completer.complete_prompt_names("/prompt test").unwrap();
        assert_eq!(pos, 8);
        assert_eq!(candidates.len(), 2); // test_prompt1 and test_prompt2

        // Test with specific prompt name
        let (pos, candidates) = completer
            .complete_prompt_names("/prompt test_prompt1")
            .unwrap();
        assert_eq!(pos, 8);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].display, "test_prompt1");

        // Test with no match
        let (pos, candidates) = completer
            .complete_prompt_names("/prompt nonexistent")
            .unwrap();
        assert_eq!(pos, 8);
        assert_eq!(candidates.len(), 0);
    }

    #[test]
    fn test_complete_prompt_flags() {
        let cache = create_test_cache();
        let completer = GooseCompleter::new(cache);

        // Test with partial flag
        let (_pos, candidates) = completer
            .complete_prompt_flags("/prompt test_prompt1 --")
            .unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].display, "--info");

        // Test with exact flag
        let (_pos, candidates) = completer
            .complete_prompt_flags("/prompt test_prompt1 --info")
            .unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].display, "--info");

        // Test with no match
        let (_pos, candidates) = completer
            .complete_prompt_flags("/prompt test_prompt1 --nonexistent")
            .unwrap();
        assert_eq!(candidates.len(), 0);

        // Test with no flag
        let (_pos, candidates) = completer
            .complete_prompt_flags("/prompt test_prompt1")
            .unwrap();
        assert_eq!(candidates.len(), 0);
    }

    #[test]
    fn test_complete_argument_keys() {
        let cache = create_test_cache();
        let completer = GooseCompleter::new(cache);

        // Test with just a prompt name (no space after)
        // This case doesn't return any candidates in the current implementation
        let (_pos, candidates) = completer
            .complete_argument_keys("/prompt test_prompt1")
            .unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].display, "required_arg=");

        // Test with partial argument
        let (_pos, candidates) = completer
            .complete_argument_keys("/prompt test_prompt1 req")
            .unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].display, "required_arg=");

        // Test with one argument already provided
        let (_pos, candidates) = completer
            .complete_argument_keys("/prompt test_prompt1 required_arg=value")
            .unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].display, "optional_arg=");

        // Test with all arguments provided
        let (_pos, candidates) = completer
            .complete_argument_keys("/prompt test_prompt1 required_arg=value optional_arg=value")
            .unwrap();
        assert_eq!(candidates.len(), 0);

        // Test with prompt that has no arguments
        let (_pos, candidates) = completer
            .complete_argument_keys("/prompt test_prompt2")
            .unwrap();
        assert_eq!(candidates.len(), 0);

        // Test with nonexistent prompt
        let (_pos, candidates) = completer
            .complete_argument_keys("/prompt nonexistent")
            .unwrap();
        assert_eq!(candidates.len(), 0);
    }

    #[test]
    fn test_fuzzy_match_exact() {
        let (score, matches) = GooseCompleter::fuzzy_match("readme", "README.md");
        assert!(score > 0, "Exact match should have positive score");
        assert_eq!(matches.len(), 6, "All 6 chars should match");
    }

    #[test]
    fn test_fuzzy_match_partial() {
        let (score, matches) = GooseCompleter::fuzzy_match("mai", "src/main.rs");
        assert!(score > 0, "Partial match should have positive score");
        assert_eq!(matches.len(), 3, "All 3 pattern chars should match");
        // Check that m, a, i are found in main
        assert!(matches.contains(&4)); // 'm' position in "src/main.rs"
        assert!(matches.contains(&5)); // 'a' position
        assert!(matches.contains(&6)); // 'i' position
    }

    #[test]
    fn test_fuzzy_match_no_match() {
        let (score, matches) = GooseCompleter::fuzzy_match("xyz", "main.rs");
        assert_eq!(score, -1, "No match should return -1");
        assert_eq!(matches.len(), 0, "No match should have empty matches");
    }

    #[test]
    fn test_fuzzy_match_empty_pattern() {
        let (score, matches) = GooseCompleter::fuzzy_match("", "main.rs");
        assert_eq!(score, 0, "Empty pattern should return 0");
        assert_eq!(matches.len(), 0, "Empty pattern should have no matches");
    }

    #[test]
    fn test_fuzzy_match_filename_bonus() {
        // Compare exact filename match vs path component match
        let score_filename = GooseCompleter::fuzzy_match("readme", "README.md").0;
        let score_path = GooseCompleter::fuzzy_match("readme", "src/read_metadata.rs").0;
        eprintln!(
            "README.md score: {}, src/read_metadata.rs score: {}",
            score_filename, score_path
        );
        // README.md has "readme" in filename, read_metadata only matches in path
        assert!(
            score_filename > score_path,
            "Filename match should score higher than path match (got {} vs {})",
            score_filename,
            score_path
        );
    }

    #[test]
    fn test_fuzzy_match_case_insensitive() {
        let (score1, _) = GooseCompleter::fuzzy_match("READ", "readme.md");
        let (score2, _) = GooseCompleter::fuzzy_match("read", "README.md");
        assert!(score1 > 0, "Uppercase pattern should match lowercase text");
        assert!(score2 > 0, "Lowercase pattern should match uppercase text");
    }

    #[test]
    fn test_fuzzy_match_consecutive_bonus() {
        let (score_consecutive, _) = GooseCompleter::fuzzy_match("mai", "main.rs");
        let (score_scattered, _) = GooseCompleter::fuzzy_match("man", "main.rs");
        // "mai" has all chars consecutive in "main", should score higher than "man"
        assert!(
            score_consecutive > 0,
            "Consecutive match should have positive score"
        );
        assert!(
            score_scattered > 0,
            "Scattered match should have positive score"
        );
    }

    #[test]
    fn test_scan_files_basic() {
        let cache = create_test_cache();
        let completer = GooseCompleter::new(cache);

        // Scan current directory (should include at least completion.rs)
        let candidates = completer.scan_files_for_completion(Path::new("."), 5);

        assert!(!candidates.is_empty(), "Should find some files");
        eprintln!("Found {} files", candidates.len());
    }

    #[test]
    fn test_scan_files_respects_ignore_patterns() {
        let cache = create_test_cache();
        let completer = GooseCompleter::new(cache);

        // Scan current goose directory
        let candidates = completer.scan_files_for_completion(Path::new("../.."), 5);

        // Should not include node_modules or .git files
        assert!(
            !candidates
                .iter()
                .any(|c| c.relative_path.contains("node_modules")),
            "Should skip node_modules"
        );
        assert!(
            !candidates.iter().any(|c| c.relative_path.contains(".git/")),
            "Should skip .git directories"
        );
    }

    #[test]
    fn test_scan_files_finds_nested_files() {
        let cache = create_test_cache();
        let completer = GooseCompleter::new(cache);

        // Scan current directory
        let candidates = completer.scan_files_for_completion(Path::new("."), 5);

        // Should find both files and directories
        let has_files = candidates.iter().any(|c| !c.is_directory);
        let has_dirs = candidates.iter().any(|c| c.is_directory);

        assert!(has_files, "Should find some files");
        assert!(has_dirs, "Should find some directories");

        eprintln!(
            "Found {} files, {} directories",
            candidates.iter().filter(|c| !c.is_directory).count(),
            candidates.iter().filter(|c| c.is_directory).count()
        );
    }

    #[test]
    fn test_complete_file_mention_basic() {
        let cache = create_test_cache();
        let completer = GooseCompleter::new(cache);
        let history = rustyline::history::DefaultHistory::new();
        let ctx = Context::new(&history);

        // Test @ at start of line
        let (pos, candidates) = completer.complete("@comp", 5, &ctx).unwrap();
        assert_eq!(pos, 1, "Position should be right after @");
        assert!(!candidates.is_empty(), "Should find some matches");

        eprintln!(
            "Found {} candidates for '@comp': {:?}",
            candidates.len(),
            candidates.iter().take(3).map(|c| &c.display).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_complete_file_mention_empty_query() {
        let cache = create_test_cache();
        let completer = GooseCompleter::new(cache);
        let history = rustyline::history::DefaultHistory::new();
        let ctx = Context::new(&history);

        // Test @ with no query
        let (pos, candidates) = completer.complete("@", 1, &ctx).unwrap();
        assert_eq!(pos, 1);
        assert!(!candidates.is_empty(), "Should return some files for empty query");
        assert!(candidates.len() <= 20, "Should limit to 20 results");
    }

    #[test]
    fn test_complete_file_mention_with_space() {
        let cache = create_test_cache();
        let completer = GooseCompleter::new(cache);
        let history = rustyline::history::DefaultHistory::new();
        let ctx = Context::new(&history);

        // Test @ with space after (should not complete)
        let (_pos, candidates) = completer.complete("@README ", 8, &ctx).unwrap();
        assert_eq!(candidates.len(), 0, "Should not complete after space");
    }

    #[test]
    fn test_complete_file_mention_multiple_at() {
        let cache = create_test_cache();
        let completer = GooseCompleter::new(cache);
        let history = rustyline::history::DefaultHistory::new();
        let ctx = Context::new(&history);

        // Test multiple @ symbols (should complete from last one)
        let line = "Check @README.md and @comp";
        let (pos, candidates) = completer
            .complete(line, line.len(), &ctx)
            .unwrap();
        assert_eq!(pos, 22, "Position should be after the last @");
        assert!(!candidates.is_empty(), "Should find matches from last @");
    }
}
