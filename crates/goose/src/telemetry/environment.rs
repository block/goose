use std::path::Path;

pub fn detect_environment() -> Option<String> {
    let mut env_indicators = Vec::new();

    if std::env::var("CI").is_ok() {
        env_indicators.push("ci");
    }
    if std::env::var("GITHUB_ACTIONS").is_ok() {
        env_indicators.push("github-actions");
    }
    if std::env::var("JENKINS_URL").is_ok() {
        env_indicators.push("jenkins");
    }
    if std::env::var("GITLAB_CI").is_ok() {
        env_indicators.push("gitlab-ci");
    }

    if std::env::var("DOCKER_CONTAINER").is_ok() || Path::new("/.dockerenv").exists() {
        env_indicators.push("docker");
    }
    if std::env::var("KUBERNETES_SERVICE_HOST").is_ok() {
        env_indicators.push("kubernetes");
    }

    if std::env::var("AWS_LAMBDA_FUNCTION_NAME").is_ok() {
        env_indicators.push("aws-lambda");
    }
    if std::env::var("GOOGLE_CLOUD_PROJECT").is_ok() {
        env_indicators.push("gcp");
    }
    if std::env::var("AZURE_FUNCTIONS_ENVIRONMENT").is_ok() {
        env_indicators.push("azure-functions");
    }

    if std::env::var("VSCODE_INJECTION").is_ok() {
        env_indicators.push("vscode");
    }
    if std::env::var("TERM_PROGRAM").as_deref() == Ok("iTerm.app") {
        env_indicators.push("iterm");
    }
    if std::env::var("TERM_PROGRAM").as_deref() == Ok("Apple_Terminal") {
        env_indicators.push("terminal-app");
    }

    if std::env::var("GOOSE_JOB_ID").is_ok() {
        env_indicators.push("scheduled");
    }

    #[cfg(target_os = "macos")]
    env_indicators.push("macos");
    #[cfg(target_os = "linux")]
    env_indicators.push("linux");
    #[cfg(target_os = "windows")]
    env_indicators.push("windows");

    #[cfg(target_arch = "x86_64")]
    env_indicators.push("x86_64");
    #[cfg(target_arch = "aarch64")]
    env_indicators.push("aarch64");
    #[cfg(target_arch = "arm")]
    env_indicators.push("arm");

    if env_indicators.is_empty() {
        None
    } else {
        Some(env_indicators.join(","))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_detect_environment_with_no_indicators() {
        let vars_to_clear = [
            "CI",
            "GITHUB_ACTIONS",
            "JENKINS_URL",
            "GITLAB_CI",
            "DOCKER_CONTAINER",
            "KUBERNETES_SERVICE_HOST",
            "AWS_LAMBDA_FUNCTION_NAME",
            "GOOGLE_CLOUD_PROJECT",
            "AZURE_FUNCTIONS_ENVIRONMENT",
            "VSCODE_INJECTION",
            "TERM_PROGRAM",
            "GOOSE_JOB_ID",
        ];

        let original_values: Vec<_> = vars_to_clear
            .iter()
            .map(|var| (var, env::var(var).ok()))
            .collect();

        for var in &vars_to_clear {
            env::remove_var(var);
        }

        let result = detect_environment();

        for (var, original_value) in original_values {
            if let Some(value) = original_value {
                env::set_var(var, value);
            }
        }

        assert!(result.is_some());
        let env_string = result.unwrap();

        #[cfg(target_os = "macos")]
        assert!(env_string.contains("macos"));
        #[cfg(target_os = "linux")]
        assert!(env_string.contains("linux"));
        #[cfg(target_os = "windows")]
        assert!(env_string.contains("windows"));

        #[cfg(target_arch = "x86_64")]
        assert!(env_string.contains("x86_64"));
        #[cfg(target_arch = "aarch64")]
        assert!(env_string.contains("aarch64"));
    }

    #[test]
    fn test_detect_environment_with_ci() {
        env::set_var("CI", "true");
        let result = detect_environment();
        assert!(result.is_some());
        assert!(result.unwrap().contains("ci"));
        env::remove_var("CI");
    }

    #[test]
    fn test_detect_environment_with_github_actions() {
        env::set_var("GITHUB_ACTIONS", "true");
        let result = detect_environment();
        assert!(result.is_some());
        assert!(result.unwrap().contains("github-actions"));
        env::remove_var("GITHUB_ACTIONS");
    }

    #[test]
    fn test_detect_environment_with_docker() {
        env::set_var("DOCKER_CONTAINER", "true");
        let result = detect_environment();
        assert!(result.is_some());
        assert!(result.unwrap().contains("docker"));
        env::remove_var("DOCKER_CONTAINER");
    }
}
