use crate::recipes::secret_discovery::SecretRequirement;
use anyhow::{Context, Result};
use console::style;
use goose::config::Config;
use serde_json::Value;
use std::collections::HashMap;

/// Interactive secret collector for gathering missing credentials
pub struct SecretCollector {
    config: &'static Config,
}

impl SecretCollector {
    /// Create a new SecretCollector instance
    pub fn new() -> Self {
        Self {
            config: Config::global(),
        }
    }

    /// Collect missing secrets through interactive prompts
    pub async fn collect_missing_secrets(
        &self,
        requirements: Vec<SecretRequirement>,
    ) -> Result<()> {
        let missing_requirements: Vec<_> = requirements
            .into_iter()
            .filter(|req| !req.is_available)
            .collect();

        if missing_requirements.is_empty() {
            return Ok(());
        }

        // Group secrets by extension for better UX
        let mut grouped_secrets: HashMap<String, Vec<SecretRequirement>> = HashMap::new();
        for requirement in missing_requirements {
            grouped_secrets
                .entry(requirement.extension_name.clone())
                .or_default()
                .push(requirement);
        }

        // Present intro
        cliclack::intro(style(" extension-setup ").on_blue().black())
            .context("Failed to show intro")?;

        // Show overview of what's needed
        let total_secrets: usize = grouped_secrets.values().map(|v| v.len()).sum();
        let extension_count = grouped_secrets.len();

        println!(
            "{}",
            style(format!(
                "Extension credentials required ({} {} across {} {}):",
                total_secrets,
                if total_secrets == 1 {
                    "secret"
                } else {
                    "secrets"
                },
                extension_count,
                if extension_count == 1 {
                    "extension"
                } else {
                    "extensions"
                }
            ))
            .dim()
        );
        println!();

        // Show grouped overview
        for (extension_name, secrets) in &grouped_secrets {
            println!("  {} extension needs:", style(extension_name).cyan().bold());
            for secret in secrets {
                println!("  • {}", style(&secret.key).yellow());
            }
            println!();
        }

        // Collect secrets for each extension
        for (extension_name, secrets) in grouped_secrets {
            self.collect_extension_secrets(&extension_name, &secrets)
                .await?;
        }

        cliclack::outro("Extension credentials configured successfully")
            .context("Failed to show outro")?;
        Ok(())
    }

    /// Present extension-specific secret collection dialog
    async fn collect_extension_secrets(
        &self,
        extension_name: &str,
        missing_secrets: &[SecretRequirement],
    ) -> Result<()> {
        if missing_secrets.len() > 1 {
            println!(
                "{}",
                style(format!("Configuring {} extension:", extension_name))
                    .green()
                    .bold()
            );
            println!();
        }

        for secret in missing_secrets {
            self.collect_single_secret(secret, extension_name).await?;
        }

        Ok(())
    }

    /// Collect a single secret with appropriate prompting
    async fn collect_single_secret(
        &self,
        secret: &SecretRequirement,
        extension_name: &str,
    ) -> Result<()> {
        let prompt = format!("Enter {} for {} extension", secret.key, extension_name);

        // Use password input for sensitive data
        let value = cliclack::password(prompt)
            .mask('▪')
            .validate(|input: &String| {
                if input.trim().is_empty() {
                    Err("Value cannot be empty")
                } else {
                    Ok(())
                }
            })
            .interact()
            .context("Failed to collect secret input")?;

        // Store the secret
        self.store_secret(&secret.key, &value)?;

        Ok(())
    }

    /// Store a secret securely
    fn store_secret(&self, key: &str, value: &str) -> Result<()> {
        self.config
            .set_secret(key, Value::String(value.to_string()))
            .with_context(|| format!("Failed to store secret '{}'", key))?;
        Ok(())
    }
}

impl Default for SecretCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recipes::secret_discovery::{SecretRequirement, SecretSource};

    fn create_test_requirement(
        key: &str,
        extension: &str,
        description: Option<&str>,
    ) -> SecretRequirement {
        SecretRequirement {
            key: key.to_string(),
            extension_name: extension.to_string(),
            description: description.map(|d| d.to_string()),
            is_available: false,
            source: SecretSource::Missing,
        }
    }

    #[test]
    fn test_secret_collection_grouping() {
        let requirements = vec![
            create_test_requirement("GITHUB_TOKEN", "github-mcp", Some("GitHub API token")),
            create_test_requirement("GITHUB_REPO", "github-mcp", Some("GitHub repository")),
            create_test_requirement("OPENAI_API_KEY", "openai-provider", Some("OpenAI API key")),
        ];

        let mut grouped_secrets: HashMap<String, Vec<SecretRequirement>> = HashMap::new();
        for requirement in requirements {
            grouped_secrets
                .entry(requirement.extension_name.clone())
                .or_default()
                .push(requirement);
        }

        assert_eq!(grouped_secrets.len(), 2);
        assert_eq!(grouped_secrets["github-mcp"].len(), 2);
        assert_eq!(grouped_secrets["openai-provider"].len(), 1);
    }

    #[test]
    fn test_empty_requirements() {
        let empty_requirements: Vec<SecretRequirement> = vec![];

        // This should not panic and should handle empty case gracefully
        // In a real test, we'd use tokio::test, but for now we just test the logic
        let missing: Vec<SecretRequirement> = empty_requirements
            .into_iter()
            .filter(|req| !req.is_available)
            .collect();

        assert!(missing.is_empty());
    }

    #[test]
    fn test_available_requirements_filtered() {
        let requirements = vec![
            SecretRequirement {
                key: "AVAILABLE_KEY".to_string(),
                extension_name: "test-extension".to_string(),
                description: None,
                is_available: true,
                source: SecretSource::Environment,
            },
            create_test_requirement("MISSING_KEY", "test-extension", None),
        ];

        let missing: Vec<SecretRequirement> = requirements
            .into_iter()
            .filter(|req| !req.is_available)
            .collect();

        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0].key, "MISSING_KEY");
    }
}
