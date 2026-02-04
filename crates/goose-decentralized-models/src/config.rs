use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

fn default_ttl() -> u64 {
    3600 // 1 hour
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub name: String,
    pub endpoint: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_size: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geo: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NostrShareConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private_key: Option<String>,
    pub relays: Vec<String>,
    pub models: Vec<ModelConfig>,
    #[serde(default = "default_ttl")]
    pub ttl_seconds: u64,
}

pub async fn detect_public_ip() -> Result<String> {
    let client = reqwest::Client::new();
    let services = [
        "http://api.ipify.org",
        "http://ifconfig.me/ip",
        "http://icanhazip.com",
    ];

    for service in services {
        if let Ok(resp) = client
            .get(service)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
        {
            if let Ok(ip) = resp.text().await {
                let ip = ip.trim().to_string();
                if !ip.is_empty() && ip.parse::<std::net::IpAddr>().is_ok() {
                    return Ok(ip);
                }
            }
        }
    }
    anyhow::bail!("Could not detect public IP")
}

fn config_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
    Ok(home.join(".config").join("goose"))
}

impl NostrShareConfig {
    pub fn default_path() -> Result<PathBuf> {
        Ok(config_dir()?.join("decentralized-models.json"))
    }

    pub fn load(path: &PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&content)?)
    }

    pub fn load_default() -> Result<Self> {
        Self::load(&Self::default_path()?)
    }

    pub fn save(&self, path: &PathBuf) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn save_default(&self) -> Result<PathBuf> {
        let path = Self::default_path()?;
        self.save(&path)?;
        Ok(path)
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_serialization() {
        let config = NostrShareConfig {
            private_key: None,
            relays: vec!["wss://relay.damus.io".to_string()],
            models: vec![ModelConfig {
                name: "qwen3".to_string(),
                endpoint: "http://192.168.1.1:11434".to_string(),
                display_name: Some("Qwen 3".to_string()),
                description: None,
                context_size: Some(32000),
                cost: Some(0.0),
                geo: Some("US".to_string()),
            }],
            ttl_seconds: 3600,
        };

        let json = serde_json::to_string(&config).unwrap();
        let parsed: NostrShareConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.models[0].name, "qwen3");
        assert_eq!(parsed.models[0].endpoint, "http://192.168.1.1:11434");
        assert_eq!(parsed.models[0].cost, Some(0.0));
        assert_eq!(parsed.models[0].geo, Some("US".to_string()));
        assert_eq!(parsed.ttl_seconds, 3600);
    }
}
