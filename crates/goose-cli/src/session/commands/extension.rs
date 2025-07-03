use anyhow::Result;
use goose::agents::extension::{Envs, ExtensionConfig};
use rand::{distributions::Alphanumeric, Rng};
use std::collections::HashMap;

use crate::session::Session;

impl Session {
    pub async fn add_extension(&mut self, extension_command: String) -> Result<()> {
        let mut parts: Vec<&str> = extension_command.split_whitespace().collect();
        let mut envs = std::collections::HashMap::new();

        // Parse environment variables (format: KEY=value)
        while let Some(part) = parts.first() {
            if !part.contains('=') {
                break;
            }
            let env_part = parts.remove(0);
            let (key, value) = env_part.split_once('=').unwrap();
            envs.insert(key.to_string(), value.to_string());
        }

        if parts.is_empty() {
            return Err(anyhow::anyhow!("No command provided in extension string"));
        }

        let cmd = parts.remove(0).to_string();
        // Generate a random name for the ephemeral extension
        let name: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(8)
            .map(char::from)
            .collect();

        let config = ExtensionConfig::Stdio {
            name,
            cmd,
            args: parts.iter().map(|s| s.to_string()).collect(),
            envs: Envs::new(envs),
            env_keys: Vec::new(),
            description: Some(goose::config::DEFAULT_EXTENSION_DESCRIPTION.to_string()),
            // TODO: should set timeout
            timeout: Some(goose::config::DEFAULT_EXTENSION_TIMEOUT),
            bundled: None,
        };

        self.agent
            .add_extension(config)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to start extension: {}", e))?;

        // Invalidate the completion cache when a new extension is added
        self.invalidate_completion_cache().await;

        Ok(())
    }

    pub async fn add_remote_extension(&mut self, extension_url: String) -> Result<()> {
        let name: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(8)
            .map(char::from)
            .collect();

        let config = ExtensionConfig::Sse {
            name,
            uri: extension_url,
            envs: Envs::new(HashMap::new()),
            env_keys: Vec::new(),
            description: Some(goose::config::DEFAULT_EXTENSION_DESCRIPTION.to_string()),
            // TODO: should set timeout
            timeout: Some(goose::config::DEFAULT_EXTENSION_TIMEOUT),
            bundled: None,
        };

        self.agent
            .add_extension(config)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to start extension: {}", e))?;

        // Invalidate the completion cache when a new extension is added
        self.invalidate_completion_cache().await;

        Ok(())
    }

    /// Add a builtin extension to the session
    ///
    /// # Arguments
    /// * `builtin_name` - Name of the builtin extension(s), comma separated
    pub async fn add_builtin(&mut self, builtin_name: String) -> Result<()> {
        for name in builtin_name.split(',') {
            let config = ExtensionConfig::Builtin {
                name: name.trim().to_string(),
                display_name: None,
                // TODO: should set a timeout
                timeout: Some(goose::config::DEFAULT_EXTENSION_TIMEOUT),
                bundled: None,
            };
            self.agent
                .add_extension(config)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to start builtin extension: {}", e))?;
        }

        // Invalidate the completion cache when a new extension is added
        self.invalidate_completion_cache().await;

        Ok(())
    }
}
