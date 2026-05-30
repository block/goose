use anyhow::{anyhow, Result};

use crate::config::Config;

use super::GooseBotPrefs;

pub const PREFS_CONFIG_KEY: &str = "goose_bot_prefs";

pub fn load_prefs_from(config: &Config) -> GooseBotPrefs {
    match config.get_param::<serde_json::Value>(PREFS_CONFIG_KEY) {
        Ok(v) => serde_json::from_value(v).unwrap_or_default(),
        Err(_) => GooseBotPrefs::default(),
    }
}

pub fn save_prefs_to(config: &Config, prefs: &GooseBotPrefs) -> Result<()> {
    config
        .set_param(PREFS_CONFIG_KEY, serde_json::to_value(prefs)?)
        .map_err(|e| anyhow!("persist goose_bot prefs: {e}"))
}

pub fn load_prefs() -> GooseBotPrefs {
    load_prefs_from(Config::global())
}

pub fn save_prefs(prefs: &GooseBotPrefs) -> Result<()> {
    save_prefs_to(Config::global(), prefs)
}

pub fn cached_installation_id(config: &Config) -> Option<u64> {
    config
        .get_param::<u64>(super::switchboard::INSTALLATION_ID_CONFIG_KEY)
        .ok()
}

pub fn clear_install(config: &Config) {
    let _ = config.delete(super::switchboard::INSTALLATION_ID_CONFIG_KEY);
    let _ = config.delete(PREFS_CONFIG_KEY);
}
