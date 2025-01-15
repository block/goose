use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

// Profile types and structures
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Profile {
    pub provider: String,
    pub model: String,
    pub temperature: Option<f32>,
    pub context_limit: Option<usize>,
    pub max_tokens: Option<i32>,
    pub estimate_factor: Option<f32>,
}

#[derive(Serialize, Deserialize)]
pub struct Profiles {
    pub profile_items: HashMap<String, Profile>,
}

pub fn profile_path() -> Result<PathBuf> {
    let home_dir = dirs::home_dir().ok_or(anyhow::anyhow!("Could not determine home directory"))?;
    let config_dir = home_dir.join(".config").join("goose");
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)?;
    }
    Ok(config_dir.join("profiles.json"))
}

pub fn load_profiles() -> Result<HashMap<String, Profile>> {
    let path = profile_path()?;
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let content = fs::read_to_string(path)?;
    let profiles: Profiles = serde_json::from_str(&content)?;
    Ok(profiles.profile_items)
}

pub fn save_profile(name: &str, profile: Profile) -> Result<()> {
    let path = profile_path()?;
    let mut profiles = load_profiles()?;
    profiles.insert(name.to_string(), profile);
    let profiles = Profiles {
        profile_items: profiles,
    };
    let content = serde_json::to_string_pretty(&profiles)?;
    fs::write(path, content)?;
    Ok(())
}

pub fn find_existing_profile(name: &str) -> Option<Profile> {
    match load_profiles() {
        Ok(profiles) => profiles.get(name).cloned(),
        Err(_) => None,
    }
}

pub fn has_no_profiles() -> Result<bool> {
    load_profiles().map(|profiles| Ok(profiles.is_empty()))?
}

pub fn set_provider_env_vars(provider_name: &str, profile: &Profile) {
    if let Some(temp) = profile.temperature {
        std::env::set_var(
            format!("{}_TEMPERATURE", provider_name.to_uppercase()),
            temp.to_string(),
        );
    }
    if let Some(limit) = profile.context_limit {
        std::env::set_var(
            format!("{}_CONTEXT_LIMIT", provider_name.to_uppercase()),
            limit.to_string(),
        );
    }
    if let Some(tokens) = profile.max_tokens {
        std::env::set_var(
            format!("{}_MAX_TOKENS", provider_name.to_uppercase()),
            tokens.to_string(),
        );
    }
    if let Some(factor) = profile.estimate_factor {
        std::env::set_var(
            format!("{}_ESTIMATE_FACTOR", provider_name.to_uppercase()),
            factor.to_string(),
        );
    }
    std::env::set_var(
        format!("{}_MODEL", provider_name.to_uppercase()),
        &profile.model,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::run_profile_with_tmp_dir;

    #[test]
    fn test_partial_profile_config() -> Result<()> {
        let profile = r#"
{
    "profile_items": {
        "default": {
            "provider": "databricks",
            "model": "claude-3",
            "temperature": 0.7,
            "context_limit": 50000
        }
    }
}
"#;
        run_profile_with_tmp_dir(profile, || {
            let profiles = load_profiles()?;
            let profile = profiles.get("default").unwrap();

            assert_eq!(profile.temperature, Some(0.7));
            assert_eq!(profile.context_limit, Some(50_000));
            assert_eq!(profile.max_tokens, None);
            assert_eq!(profile.estimate_factor, None);

            // Skip provider creation test since it requires environment variables
            Ok(())
        })
    }
}
