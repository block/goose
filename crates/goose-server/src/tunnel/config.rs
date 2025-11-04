use super::TunnelConfig;
use goose::config::Config;
use tracing::info;

const CONFIG_KEY_MODE: &str = "tunnel_mode";
const CONFIG_KEY_AUTO_START: &str = "tunnel_auto_start";
const SECRET_KEY_SECRET: &str = "tunnel_secret";
const SECRET_KEY_AGENT_ID: &str = "tunnel_agent_id";

/// Load tunnel configuration from Config system
/// Non-secrets from config.yaml, secrets from keyring/secrets.yaml
pub async fn load_config() -> TunnelConfig {
    let cfg = Config::global();

    let mode = cfg
        .get_param(CONFIG_KEY_MODE)
        .unwrap_or_else(|_| super::TunnelMode::default());

    let auto_start = cfg.get_param(CONFIG_KEY_AUTO_START).unwrap_or(false);

    let secret = cfg.get_secret(SECRET_KEY_SECRET).ok();
    let agent_id = cfg.get_secret(SECRET_KEY_AGENT_ID).ok();

    info!(
        "Loaded tunnel config from goose Config (mode: {:?}, auto_start: {})",
        mode, auto_start
    );

    TunnelConfig {
        mode,
        auto_start,
        secret,
        agent_id,
    }
}

/// Save tunnel configuration to Config system
/// Non-secrets to config.yaml, secrets to keyring/secrets.yaml
pub async fn save_config(config: &TunnelConfig) -> Result<(), Box<dyn std::error::Error>> {
    let cfg = Config::global();

    // Save non-secret config to config.yaml
    cfg.set_param(CONFIG_KEY_MODE, &config.mode)?;
    cfg.set_param(CONFIG_KEY_AUTO_START, config.auto_start)?;

    // Save secrets to keyring or secrets.yaml
    if let Some(secret) = &config.secret {
        cfg.set_secret(SECRET_KEY_SECRET, secret)?;
    }
    if let Some(agent_id) = &config.agent_id {
        cfg.set_secret(SECRET_KEY_AGENT_ID, agent_id)?;
    }

    info!("Saved tunnel config to goose Config");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tunnel::TunnelMode;

    fn clear_secrets() -> Result<(), Box<dyn std::error::Error>> {
        let cfg = Config::global();
        let _ = cfg.delete_secret(SECRET_KEY_SECRET);
        let _ = cfg.delete_secret(SECRET_KEY_AGENT_ID);
        Ok(())
    }

    #[tokio::test]
    async fn test_save_and_load_config() {
        let config = TunnelConfig {
            auto_start: true,
            mode: TunnelMode::Lapstone,
            secret: Some("test_secret_12345".to_string()),
            agent_id: Some("test_agent_id_67890".to_string()),
        };

        // Save config
        save_config(&config).await.unwrap();

        // Load config
        let loaded = load_config().await;

        assert_eq!(loaded.auto_start, config.auto_start);
        assert_eq!(loaded.mode, config.mode);
        assert_eq!(loaded.secret, config.secret);
        assert_eq!(loaded.agent_id, config.agent_id);

        // Cleanup
        clear_secrets().unwrap();
    }

    #[tokio::test]
    async fn test_load_config_defaults() {
        // Clear any existing config
        let _ = clear_secrets();

        let cfg = Config::global();
        let _ = cfg.delete(CONFIG_KEY_MODE);
        let _ = cfg.delete(CONFIG_KEY_AUTO_START);

        // Load should return defaults
        let loaded = load_config().await;

        assert!(!loaded.auto_start);
        assert_eq!(loaded.mode, TunnelMode::default());
        assert_eq!(loaded.secret, None);
        assert_eq!(loaded.agent_id, None);
    }
}
