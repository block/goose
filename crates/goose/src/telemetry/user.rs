use crate::telemetry::config::UsageType;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserIdentity {
    pub user_id: String,
    pub usage_type: UsageType,
    pub first_seen: chrono::DateTime<chrono::Utc>,
    pub last_seen: chrono::DateTime<chrono::Utc>,
}

impl UserIdentity {
    pub async fn load_or_create() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let identity_path = get_identity_file_path()?;

        if identity_path.exists() {
            match fs::read_to_string(&identity_path) {
                Ok(content) => {
                    if let Ok(mut identity) = serde_json::from_str::<UserIdentity>(&content) {
                        identity.last_seen = chrono::Utc::now();
                        identity.usage_type = detect_usage_type();

                        if let Err(e) = identity.save().await {
                            tracing::warn!("Failed to update user identity: {}", e);
                        }

                        return Ok(identity);
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to read user identity file: {}", e);
                }
            }
        }

        let identity = Self::new();
        if let Err(e) = identity.save().await {
            tracing::warn!("Failed to save new user identity: {}", e);
        }

        Ok(identity)
    }

    fn new() -> Self {
        let now = chrono::Utc::now();
        Self {
            user_id: Uuid::new_v4().to_string(),
            usage_type: detect_usage_type(),
            first_seen: now,
            last_seen: now,
        }
    }

    async fn save(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let identity_path = get_identity_file_path()?;

        if let Some(parent) = identity_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        fs::write(&identity_path, content)?;

        Ok(())
    }
}

pub fn detect_usage_type() -> UsageType {
    if let Ok(usage_type) = env::var("GOOSE_USAGE_TYPE") {
        if let Ok(parsed) = usage_type.parse::<UsageType>() {
            return parsed;
        }
    }

    if is_ci_environment() {
        return UsageType::Ci;
    }

    if is_automation_environment() {
        return UsageType::Automation;
    }

    UsageType::Human
}

fn is_ci_environment() -> bool {
    let ci_vars = [
        "CI",
        "CONTINUOUS_INTEGRATION",
        "BUILD_NUMBER",
        "GITHUB_ACTIONS",
        "GITLAB_CI",
        "JENKINS_URL",
        "TRAVIS",
        "CIRCLECI",
        "BUILDKITE",
        "DRONE",
        "TEAMCITY_VERSION",
        "TF_BUILD",
        "CODEBUILD_BUILD_ID",
    ];

    ci_vars.iter().any(|var| env::var(var).is_ok())
}

fn is_automation_environment() -> bool {
    let automation_vars = [
        "AWS_LAMBDA_FUNCTION_NAME",
        "KUBERNETES_SERVICE_HOST",
        "DOCKER_CONTAINER",
        "SERVERLESS",
        "FUNCTION_NAME",
        "AZURE_FUNCTIONS_ENVIRONMENT",
        "GOOSE_AUTOMATION",
        "GOOSE_JOB_ID", // Temporal service scheduled jobs
    ];

    automation_vars.iter().any(|var| env::var(var).is_ok())
}

fn get_identity_file_path() -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    let config_dir = dirs::config_dir().ok_or("Could not determine config directory")?;

    Ok(config_dir
        .join("goose")
        .join("telemetry")
        .join("user-identity.json"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::TempDir;

    #[test]
    fn test_detect_usage_type_default() {
        let vars_to_clear = [
            "GOOSE_USAGE_TYPE",
            "CI",
            "CONTINUOUS_INTEGRATION",
            "BUILD_NUMBER",
            "GITHUB_ACTIONS",
            "GITLAB_CI",
            "JENKINS_URL",
            "TRAVIS",
            "CIRCLECI",
            "BUILDKITE",
            "DRONE",
            "TEAMCITY_VERSION",
            "TF_BUILD",
            "CODEBUILD_BUILD_ID",
            "AWS_LAMBDA_FUNCTION_NAME",
            "GOOSE_JOB_ID",
        ];
        for var in &vars_to_clear {
            env::remove_var(var);
        }

        assert_eq!(detect_usage_type(), UsageType::Human);
    }

    #[test]
    fn test_detect_usage_type_temporal_service() {
        // Clear all environment variables first
        let vars_to_clear = [
            "GOOSE_USAGE_TYPE",
            "CI",
            "GITHUB_ACTIONS",
            "AWS_LAMBDA_FUNCTION_NAME",
            "GOOSE_JOB_ID",
        ];
        for var in &vars_to_clear {
            env::remove_var(var);
        }

        // Set temporal service job ID
        env::set_var("GOOSE_JOB_ID", "scheduled-job-abc123");
        assert_eq!(detect_usage_type(), UsageType::Automation);

        // Clean up
        env::remove_var("GOOSE_JOB_ID");
    }

    #[test]
    fn test_detect_usage_type_override() {
        env::set_var("GOOSE_USAGE_TYPE", "automation");
        assert_eq!(detect_usage_type(), UsageType::Automation);
        env::remove_var("GOOSE_USAGE_TYPE");
    }

    #[test]
    fn test_detect_ci_environment() {
        env::remove_var("CI");
        env::remove_var("GITHUB_ACTIONS");
        assert!(!is_ci_environment());

        env::set_var("CI", "true");
        assert!(is_ci_environment());
        env::remove_var("CI");

        env::set_var("GITHUB_ACTIONS", "true");
        assert!(is_ci_environment());
        env::remove_var("GITHUB_ACTIONS");
    }

    #[test]
    fn test_detect_automation_environment() {
        env::remove_var("AWS_LAMBDA_FUNCTION_NAME");
        env::remove_var("KUBERNETES_SERVICE_HOST");
        env::remove_var("GOOSE_JOB_ID");
        assert!(!is_automation_environment());

        env::set_var("AWS_LAMBDA_FUNCTION_NAME", "test-function");
        assert!(is_automation_environment());
        env::remove_var("AWS_LAMBDA_FUNCTION_NAME");

        env::set_var("KUBERNETES_SERVICE_HOST", "10.0.0.1");
        assert!(is_automation_environment());
        env::remove_var("KUBERNETES_SERVICE_HOST");

        // Test temporal service detection
        env::set_var("GOOSE_JOB_ID", "scheduled-job-123");
        assert!(is_automation_environment());
        env::remove_var("GOOSE_JOB_ID");
    }

    #[test]
    fn test_user_identity_creation() {
        let identity = UserIdentity::new();

        assert!(Uuid::parse_str(&identity.user_id).is_ok());

        let now = chrono::Utc::now();
        assert!(identity.first_seen <= now);
        assert!(identity.last_seen <= now);
        assert!(identity.first_seen <= identity.last_seen);
    }

    #[tokio::test]
    async fn test_user_identity_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let identity_path = temp_dir.path().join("user-identity.json");

        let original_identity = UserIdentity::new();
        let json = serde_json::to_string(&original_identity).unwrap();

        let loaded_identity: UserIdentity = serde_json::from_str(&json).unwrap();
        assert_eq!(original_identity.user_id, loaded_identity.user_id);
        assert_eq!(original_identity.usage_type, loaded_identity.usage_type);
    }
}
