//! Environment detection for determining if running in Docker sandbox
//! or on a real filesystem

use crate::approval::Environment;
use std::path::Path;

/// Detect the current execution environment
pub fn detect_environment() -> Environment {
    if is_running_in_docker() {
        Environment::DockerSandbox
    } else {
        Environment::RealFilesystem
    }
}

/// Check if running inside a Docker container
/// Uses multiple heuristics for detection
pub fn is_running_in_docker() -> bool {
    // Check for .dockerenv file (older Docker versions)
    if Path::new("/.dockerenv").exists() {
        return true;
    }

    // Check cgroup for docker references
    if let Ok(cgroup) = std::fs::read_to_string("/proc/self/cgroup") {
        if cgroup.contains("docker") || cgroup.contains("containerd") {
            return true;
        }
    }

    // Check for container-specific env vars
    if std::env::var("CONTAINER").is_ok() || std::env::var("KUBERNETES_SERVICE_HOST").is_ok() {
        return true;
    }

    false
}

/// Check if running in a CI environment
pub fn is_running_in_ci() -> bool {
    std::env::var("CI").is_ok()
        || std::env::var("CONTINUOUS_INTEGRATION").is_ok()
        || std::env::var("GITHUB_ACTIONS").is_ok()
        || std::env::var("GITLAB_CI").is_ok()
        || std::env::var("TRAVIS").is_ok()
        || std::env::var("CIRCLECI").is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_environment_enum() {
        assert_eq!(detect_environment(), Environment::RealFilesystem);
    }

    #[test]
    fn test_is_running_in_docker() {
        // This test will pass in both docker and non-docker environments
        // as it just checks the function doesn't panic
        let _ = is_running_in_docker();
    }

    #[test]
    fn test_is_running_in_ci() {
        // This test will pass in both CI and non-CI environments
        let _ = is_running_in_ci();
    }
}
