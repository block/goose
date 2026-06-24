use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum TunnelState {
    #[default]
    Idle,
    Starting,
    Running,
    Error,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TunnelInfo {
    pub state: TunnelState,
    pub url: String,
    pub hostname: String,
    pub secret: String,
}

pub struct TunnelManager;

impl Default for TunnelManager {
    fn default() -> Self {
        Self::new(true)
    }
}

impl TunnelManager {
    pub fn new(_tls: bool) -> Self {
        TunnelManager
    }

    fn is_tunnel_disabled() -> bool {
        if let Ok(val) = std::env::var("GOOSE_TUNNEL") {
            let val = val.to_lowercase();
            val == "no" || val == "none"
        } else {
            false
        }
    }

    pub async fn get_info(&self) -> TunnelInfo {
        TunnelInfo {
            state: if Self::is_tunnel_disabled() {
                TunnelState::Disabled
            } else {
                TunnelState::Idle
            },
            url: String::new(),
            hostname: String::new(),
            secret: String::new(),
        }
    }
}
