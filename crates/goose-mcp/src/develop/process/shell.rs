//! Shell execution logic: POSIX detection, environment tracking.

use std::collections::{HashMap, HashSet};
use std::env;
use std::path::PathBuf;

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
    /// Returns (wrapper_prefix, wrapper_suffix, delimiter).
    pub fn wrap_command(command: &str) -> (String, String, &'static str) {
        let delimiter = "__GOOSE_ENV_MARKER__";

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
        (wrapped, String::new(), delimiter)
    }

    /// Parse the output from a wrapped command, extracting:
    /// - The actual command output
    /// - New cwd
    /// - Environment changes
    /// - Exit code
    pub fn parse_wrapped_output(
        &self,
        raw_output: &str,
        delimiter: &str,
    ) -> (
        String,
        Option<PathBuf>,
        HashMap<String, String>,
        HashSet<String>,
        i32,
    ) {
        let parts: Vec<&str> = raw_output.split(delimiter).collect();

        if parts.len() < 4 {
            // Parsing failed, return raw output with no state changes
            return (
                raw_output.to_string(),
                None,
                HashMap::new(),
                HashSet::new(),
                1,
            );
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

        (command_output, new_cwd_path, changed, unset, exit_code)
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
}
