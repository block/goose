use super::base::Config;
use anyhow::Result;
use std::collections::HashMap;

const ALL_EXPERIMENTS: &[(&str, bool)] = &[
    // TODO(yingjiehe): Cleanup EXPERIMENT_CONFIG once experiment is fully ready.
    ("EXPERIMENT_CONFIG", false),
];

/// Experiment configuration management
pub struct ExperimentManager;

impl ExperimentManager {
    /// Get all experiments and their configurations
    pub fn get_all() -> Result<Vec<(String, bool)>> {
        let config = Config::global();
        let experiments: HashMap<String, bool> = config.get("experiments").unwrap_or_default();
        if experiments.is_empty() {
            Ok(experiments.iter().map(|(k, v)| (k.clone(), *v)).collect())
        } else {
            Ok(ALL_EXPERIMENTS
                .iter()
                .map(|(k, v)| (k.to_string(), *v))
                .collect())
        }
    }

    /// Enable or disable an experiment
    pub fn set_enabled(name: &str, enabled: bool) -> Result<()> {
        let config = Config::global();

        // Load existing experiments or initialize a new map
        let mut experiments: HashMap<String, bool> =
            config.get("experiments").unwrap_or_else(|_| HashMap::new());

        // Update the status of the experiment
        experiments.insert(name.to_string(), enabled);

        // Save the updated experiments map
        config.set("experiments", serde_json::to_value(experiments)?)?;
        Ok(())
    }

    /// Check if an experiment is enabled
    pub fn is_enabled(name: &str) -> Result<bool> {
        let config = Config::global();

        // Load existing experiments or initialize a new map
        let experiments: HashMap<String, bool> =
            config.get("experiments").unwrap_or_else(|_| HashMap::new());

        // Return whether the experiment is enabled, defaulting to false
        Ok(*experiments.get(name).unwrap_or(&false))
    }
}
