use super::TunnelConfig;
use goose::config::Config;
use tracing::info;

const CONFIG_KEY_AUTO_START: &str = "tunnel_auto_start";
const SECRET_KEY_SECRET: &str = "tunnel_secret";
const SECRET_KEY_AGENT_ID: &str = "tunnel_agent_id";

/// Load tunnel configuration from Config system
/// Non-secrets from config.yaml, secrets from keyring/secrets.yaml
pub async fn load_config() -> TunnelConfig {
    let cfg = Config::global();

    let auto_start = cfg.get_param(CONFIG_KEY_AUTO_START).unwrap_or(false);

    let secret = cfg.get_secret(SECRET_KEY_SECRET).ok();
    let agent_id = cfg.get_secret(SECRET_KEY_AGENT_ID).ok();

    info!(
        "Loaded tunnel config from goose Config (auto_start: {})",
        auto_start
    );

    TunnelConfig {
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
