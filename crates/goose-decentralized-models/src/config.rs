use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

fn default_ttl() -> u64 {
    3600 // 1 hour
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_size: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvertiseEndpoint {
    pub host: String,
    pub port: u16,
    #[serde(default)]
    pub https: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NostrShareConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private_key: Option<String>,
    pub relays: Vec<String>,
    pub models: Vec<ModelConfig>,
    pub ollama_endpoint: String,
    pub advertise_endpoint: AdvertiseEndpoint,
    #[serde(default = "default_ttl")]
    pub ttl_seconds: u64,
}

pub async fn detect_public_ip() -> Result<String> {
    let client = reqwest::Client::new();
    let services = [
        "https://api.ipify.org",
        "https://ifconfig.me/ip",
        "https://icanhazip.com",
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

    pub async fn resolve_endpoint(&self) -> Result<AdvertiseEndpoint> {
        let mut endpoint = self.advertise_endpoint.clone();
        if endpoint.host == "auto" || endpoint.host == "YOUR_IP" {
            endpoint.host = detect_public_ip().await?;
        }
        Ok(endpoint)
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
                display_name: Some("Qwen 3".to_string()),
                description: None,
                context_size: Some(32000),
            }],
            ollama_endpoint: "http://localhost:11434".to_string(),
            advertise_endpoint: AdvertiseEndpoint {
                host: "auto".to_string(),
                port: 11434,
                https: false,
            },
            ttl_seconds: 3600,
        };

        let json = serde_json::to_string(&config).unwrap();
        let parsed: NostrShareConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.models[0].name, "qwen3");
        assert_eq!(parsed.ttl_seconds, 3600);
    }
}
