//! Configuration file protection for developer tools
//!
//! This module prevents goose from modifying its own configuration files through
//! developer extension tools like text_editor and shell.

use std::path::{Path, PathBuf};

/// Check if a path is within the goose configuration directory
pub fn is_goose_config_path(path: &Path) -> bool {
    // Get the canonical config directory
    let config_dir = match goose::config::paths::Paths::config_dir().canonicalize() {
        Ok(dir) => dir,
        Err(_) => return false,
    };

    // Try to canonicalize the target path
    let canonical_path = match path.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            // If the file doesn't exist yet, check its parent directory
            if let Some(parent) = path.parent() {
                match parent.canonicalize() {
                    Ok(p) => p,
                    Err(_) => return false,
                }
            } else {
                return false
            }
        }
    };

    // Check if the path is within the config directory
    canonical_path.starts_with(&config_dir)
}

/// Check if a shell command might modify goose configuration files
pub fn command_touches_config(command: &str) -> bool {
    let config_dir = goose::config::paths::Paths::config_dir();
    let config_dir_str = config_dir.to_string_lossy();
    
    // Check for common patterns that might modify config files
    let dangerous_patterns = [
        "config.yaml",
        "secrets.yaml",
        ".config/goose",
        &config_dir_str,
    ];

    let command_lower = command.to_lowercase();
    
    // Check if command contains config-related paths
    for pattern in &dangerous_patterns {
        if command_lower.contains(&pattern.to_lowercase()) {
            // Check if it's a write operation
            if command_lower.contains('>') 
                || command_lower.contains("echo")
                || command_lower.contains("cat")
                || command_lower.contains("tee")
                || command_lower.contains("sed")
                || command_lower.contains("awk")
                || command_lower.contains("rm")
                || command_lower.contains("mv")
                || command_lower.contains("cp")
                || command_lower.contains("write")
                || command_lower.contains("truncate")
            {
                return true;
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_is_goose_config_path() {
        let config_dir = goose::config::paths::Paths::config_dir();
        
        // Test config.yaml
        let config_yaml = config_dir.join("config.yaml");
        assert!(is_goose_config_path(&config_yaml));
        
        // Test secrets.yaml
        let secrets_yaml = config_dir.join("secrets.yaml");
        assert!(is_goose_config_path(&secrets_yaml));
        
        // Test any file in config dir
        let some_file = config_dir.join("some_file.txt");
        assert!(is_goose_config_path(&some_file));
        
        // Test file outside config dir
        let temp_dir = TempDir::new().unwrap();
        let outside_file = temp_dir.path().join("test.txt");
        fs::write(&outside_file, "test").unwrap();
        assert!(!is_goose_config_path(&outside_file));
    }

    #[test]
    fn test_command_touches_config_write_operations() {
        // Dangerous commands
        assert!(command_touches_config("echo 'malicious' > ~/.config/goose/config.yaml"));
        assert!(command_touches_config("cat data >> ~/.config/goose/config.yaml"));
        assert!(command_touches_config("sed -i 's/old/new/' ~/.config/goose/config.yaml"));
        assert!(command_touches_config("rm ~/.config/goose/config.yaml"));
        assert!(command_touches_config("mv config.yaml ~/.config/goose/config.yaml"));
        assert!(command_touches_config("cp backup.yaml ~/.config/goose/config.yaml"));
        
        // Safe commands (read-only)
        assert!(!command_touches_config("cat ~/.config/goose/config.yaml"));
        assert!(!command_touches_config("ls ~/.config/goose/"));
        assert!(!command_touches_config("grep something ~/.config/goose/config.yaml"));
        
        // Commands with no config reference
        assert!(!command_touches_config("echo 'hello' > /tmp/test.txt"));
        assert!(!command_touches_config("ls -la"));
    }

    #[test]
    fn test_command_touches_config_case_insensitive() {
        assert!(command_touches_config("ECHO 'test' > ~/.config/goose/CONFIG.YAML"));
        assert!(command_touches_config("Echo 'test' > ~/.CONFIG/GOOSE/config.yaml"));
    }

    #[test]
    fn test_command_touches_config_secrets() {
        assert!(command_touches_config("echo 'secret' > ~/.config/goose/secrets.yaml"));
        assert!(command_touches_config("rm ~/.config/goose/secrets.yaml"));
    }
}
