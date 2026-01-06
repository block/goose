//! Shell execution logic: POSIX detection, environment tracking.

use std::collections::{HashMap, HashSet};
use std::env;
use std::fmt;
use std::path::PathBuf;

use uuid::Uuid;

/// Error type for shell output parsing failures.
#[derive(Debug, Clone)]
pub enum ParseError {
    /// The delimiter appeared an unexpected number of times in the output.
    /// This likely means the command output contained the delimiter pattern.
    DelimiterMismatch { expected: usize, found: usize },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::DelimiterMismatch { expected, found } => {
                write!(
                    f,
                    "Command output parsing failed: expected {} delimiter occurrences but found {}. \
                    This is unexpected since the delimiter uses a UUID. \
                    The command output may be corrupted.",
                    expected, found
                )
            }
        }
    }
}

impl std::error::Error for ParseError {}

/// Result of parsing wrapped shell output.
/// Contains: (command_output, new_cwd, changed_env_vars, unset_env_vars, exit_code)
pub type ParsedOutput = (
    String,
    Option<PathBuf>,
    HashMap<String, String>,
    HashSet<String>,
    i32,
);

/// Environment variables to ignore when tracking changes.
/// These change on every command or are internal shell state.
const ENV_IGNORELIST: &[&str] = &[
    "_",            // Last command
    "SHLVL",        // Shell nesting level
    "OLDPWD",       // Previous directory
    "PWD",          // We track cwd separately
    "RANDOM",       // Random number
    "LINENO",       // Line number in script
    "SECONDS",      // Seconds since shell start
    "COLUMNS",      // Terminal width
    "LINES",        // Terminal height
    "HISTCMD",      // History command number
    "BASH_COMMAND", // Current command
    "PIPESTATUS",   // Exit statuses of pipeline
];

/// POSIX-compatible shells we can use directly.
const POSIX_SHELLS: &[&str] = &["bash", "zsh", "sh", "dash", "ksh", "ash"];

/// Fallback shell when user's shell is not POSIX-compatible.
const FALLBACK_SHELL: &str = "zsh";

/// Detects the appropriate shell to use.
pub struct ShellConfig {
    pub shell_path: String,
    pub is_fallback: bool,
    pub original_shell: Option<String>,
}

impl ShellConfig {
    pub fn detect() -> Self {
        let user_shell = env::var("SHELL").unwrap_or_default();

        if Self::is_posix_shell(&user_shell) {
            Self {
                shell_path: user_shell,
                is_fallback: false,
                original_shell: None,
            }
        } else {
            Self {
                shell_path: FALLBACK_SHELL.to_string(),
                is_fallback: true,
                original_shell: Some(user_shell),
            }
        }
    }

    fn is_posix_shell(shell: &str) -> bool {
        POSIX_SHELLS
            .iter()
            .any(|&s| shell.ends_with(&format!("/{}", s)) || shell == s)
    }

    pub fn fallback_warning(&self) -> Option<String> {
        if self.is_fallback {
            Some(format!(
                "Your shell ({}) is not POSIX-compatible. Using {} for command execution. \
                Aliases and shell-specific configuration won't be available.",
                self.original_shell.as_deref().unwrap_or("unknown"),
                self.shell_path
            ))
        } else {
            None
        }
    }
}

/// Tracks environment state across commands.
#[derive(Debug, Clone)]
pub struct EnvState {
    /// Current working directory.
    pub cwd: PathBuf,
    /// Environment variable overrides (changes from initial state).
    pub vars: HashMap<String, String>,
    /// Variables that have been unset.
    pub unset: HashSet<String>,
}

impl Default for EnvState {
    fn default() -> Self {
        Self::new()
    }
}

impl EnvState {
    pub fn new() -> Self {
        Self {
            cwd: env::current_dir().unwrap_or_else(|_| PathBuf::from("/")),
            vars: HashMap::new(),
            unset: HashSet::new(),
        }
    }

    /// Generate shell commands to capture environment before/after user command.
    /// Returns (wrapped_command, delimiter).
    ///
    /// The delimiter is a UUID to avoid collisions with command output.
    pub fn wrap_command(command: &str) -> (String, String) {
        let delimiter = format!("__GOOSE_ENV_{}__", Uuid::new_v4().as_simple());

        let prefix = r#"__goose_env_before=$(env | sort)
__goose_cwd_before=$(pwd)
"#
        .to_string();

        let suffix = format!(
            r#"
__goose_exit_code=$?
echo "{delimiter}"
pwd
echo "{delimiter}"
env | sort
echo "{delimiter}"
echo $__goose_exit_code"#,
            delimiter = delimiter
        );

        let wrapped = format!("{}{}{}", prefix, command, suffix);
        (wrapped, delimiter)
    }

    /// Parse the output from a wrapped command, extracting:
    /// - The actual command output
    /// - New cwd
    /// - Environment changes
    /// - Exit code
    ///
    /// Returns Err if the delimiter appears an unexpected number of times (should be exactly 3).
    pub fn parse_wrapped_output(
        &self,
        raw_output: &str,
        delimiter: &str,
    ) -> Result<ParsedOutput, ParseError> {
        let delimiter_count = raw_output.matches(delimiter).count();

        if delimiter_count != 3 {
            return Err(ParseError::DelimiterMismatch {
                expected: 3,
                found: delimiter_count,
            });
        }

        let parts: Vec<&str> = raw_output.split(delimiter).collect();

        // With exactly 3 delimiters, we should have exactly 4 parts
        if parts.len() != 4 {
            return Err(ParseError::DelimiterMismatch {
                expected: 3,
                found: delimiter_count,
            });
        }

        let command_output = parts[0].trim_end().to_string();
        let new_cwd = parts[1].trim();
        let new_env_raw = parts[2].trim();
        let exit_code: i32 = parts[3].trim().parse().unwrap_or(1);

        // Parse new cwd
        let new_cwd_path = if !new_cwd.is_empty() && new_cwd != self.cwd.to_string_lossy() {
            Some(PathBuf::from(new_cwd))
        } else {
            None
        };

        // Parse environment and compute diff
        let new_env: HashMap<String, String> = new_env_raw
            .lines()
            .filter_map(|line| {
                let mut parts = line.splitn(2, '=');
                match (parts.next(), parts.next()) {
                    (Some(k), Some(v)) => Some((k.to_string(), v.to_string())),
                    _ => None,
                }
            })
            .collect();

        // Get current full environment for comparison
        let current_env: HashMap<String, String> = env::vars().collect();

        // Find changes (new or modified vars)
        let mut changed: HashMap<String, String> = HashMap::new();
        let mut unset: HashSet<String> = HashSet::new();

        let ignorelist: HashSet<&str> = ENV_IGNORELIST.iter().copied().collect();

        for (k, v) in &new_env {
            if ignorelist.contains(k.as_str()) {
                continue;
            }
            // Check if this is different from current env or our tracked state
            let current_val = self.vars.get(k).or_else(|| current_env.get(k));
            if current_val != Some(v) {
                changed.insert(k.clone(), v.clone());
            }
        }

        // Find unset vars (in current env or our tracked state, but not in new env)
        for k in self.vars.keys() {
            if !new_env.contains_key(k) && !ignorelist.contains(k.as_str()) {
                unset.insert(k.clone());
            }
        }

        Ok((command_output, new_cwd_path, changed, unset, exit_code))
    }

    /// Apply changes from a command execution.
    pub fn apply_changes(
        &mut self,
        new_cwd: Option<PathBuf>,
        changed: HashMap<String, String>,
        unset: HashSet<String>,
    ) {
        if let Some(cwd) = new_cwd {
            self.cwd = cwd;
        }

        for (k, v) in changed {
            self.unset.remove(&k);
            self.vars.insert(k, v);
        }

        for k in unset {
            self.vars.remove(&k);
            self.unset.insert(k);
        }
    }

    /// Generate environment setup commands for the next shell invocation.
    pub fn setup_commands(&self) -> String {
        let mut cmds = Vec::new();

        for (k, v) in &self.vars {
            // Escape single quotes in value
            let escaped = v.replace('\'', "'\\''");
            cmds.push(format!("export {}='{}'", k, escaped));
        }

        for k in &self.unset {
            cmds.push(format!("unset {}", k));
        }

        if cmds.is_empty() {
            String::new()
        } else {
            format!("{}\n", cmds.join("\n"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_detection_bash() {
        // Test the is_posix_shell function directly to avoid env var issues
        assert!(ShellConfig::is_posix_shell("/bin/bash"));
        assert!(ShellConfig::is_posix_shell("/usr/bin/zsh"));
        assert!(ShellConfig::is_posix_shell("bash"));
        assert!(ShellConfig::is_posix_shell("/bin/sh"));
    }

    #[test]
    fn test_shell_detection_fish_fallback() {
        // Test the is_posix_shell function directly
        assert!(!ShellConfig::is_posix_shell("/usr/bin/fish"));
        assert!(!ShellConfig::is_posix_shell("fish"));
        assert!(!ShellConfig::is_posix_shell("/usr/bin/nushell"));
    }

    #[test]
    fn test_env_state_setup_commands() {
        let mut state = EnvState::new();
        state.vars.insert("FOO".to_string(), "bar".to_string());
        state.vars.insert("BAZ".to_string(), "qux".to_string());

        let setup = state.setup_commands();
        assert!(setup.contains("export FOO='bar'"));
        assert!(setup.contains("export BAZ='qux'"));
    }

    #[test]
    fn test_env_state_escaping() {
        let mut state = EnvState::new();
        state
            .vars
            .insert("QUOTED".to_string(), "it's a test".to_string());

        let setup = state.setup_commands();
        assert!(setup.contains("export QUOTED='it'\\''s a test'"));
    }

    #[test]
    fn test_wrap_command_generates_uuid_delimiter() {
        let (wrapped1, delim1) = EnvState::wrap_command("echo hello");
        let (_wrapped2, delim2) = EnvState::wrap_command("echo hello");

        // Each call should generate a unique delimiter
        assert_ne!(delim1, delim2);

        // Delimiter should have the expected format
        assert!(delim1.starts_with("__GOOSE_ENV_"));
        assert!(delim1.ends_with("__"));

        // Wrapped command should contain the delimiter
        assert!(wrapped1.contains(&delim1));
    }

    #[test]
    fn test_parse_wrapped_output_success() {
        let state = EnvState::new();
        let delimiter = "__GOOSE_ENV_test123__";
        let raw_output = format!(
            "hello world\n{}\n/tmp\n{}\nFOO=bar\n{}\n0",
            delimiter, delimiter, delimiter
        );

        let result = state.parse_wrapped_output(&raw_output, delimiter);
        assert!(result.is_ok());

        let (output, new_cwd, _changed, _unset, exit_code) = result.unwrap();
        assert_eq!(output, "hello world");
        assert_eq!(new_cwd, Some(PathBuf::from("/tmp")));
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_parse_wrapped_output_delimiter_mismatch() {
        let state = EnvState::new();
        let delimiter = "__GOOSE_ENV_test123__";

        // Output with only 2 delimiters (missing one)
        let raw_output = format!("hello world\n{}\n/tmp\n{}\n0", delimiter, delimiter);

        let result = state.parse_wrapped_output(&raw_output, delimiter);
        assert!(result.is_err());

        match result {
            Err(ParseError::DelimiterMismatch { expected, found }) => {
                assert_eq!(expected, 3);
                assert_eq!(found, 2);
            }
            Ok(_) => panic!("Expected delimiter mismatch error"),
        }
    }

    #[test]
    fn test_parse_wrapped_output_extra_delimiter_in_output() {
        let state = EnvState::new();
        let delimiter = "__GOOSE_ENV_test123__";

        // Output where the command itself printed the delimiter (4 total)
        let raw_output = format!(
            "hello {} world\n{}\n/tmp\n{}\nFOO=bar\n{}\n0",
            delimiter, delimiter, delimiter, delimiter
        );

        let result = state.parse_wrapped_output(&raw_output, delimiter);
        assert!(result.is_err());

        match result {
            Err(ParseError::DelimiterMismatch { expected, found }) => {
                assert_eq!(expected, 3);
                assert_eq!(found, 4);
            }
            Ok(_) => panic!("Expected delimiter mismatch error"),
        }
    }
}
