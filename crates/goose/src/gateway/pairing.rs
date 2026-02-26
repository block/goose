use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::config::Config;

use super::{PairingState, PlatformUser};

const PAIRINGS_CONFIG_KEY: &str = "gateway_pairings";
const PENDING_CODES_CONFIG_KEY: &str = "gateway_pending_codes";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredPairing {
    platform: String,
    user_id: String,
    display_name: Option<String>,
    state: PairingState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredPendingCode {
    code: String,
    gateway_type: String,
    expires_at: i64,
}

pub struct PairingStore {
    pairings: RwLock<HashMap<PlatformUser, PairingState>>,
}

impl PairingStore {
    pub fn new() -> anyhow::Result<Self> {
        let pairings = Self::load_pairings_from_config();
        Ok(Self {
            pairings: RwLock::new(pairings),
        })
    }

    fn load_pairings_from_config() -> HashMap<PlatformUser, PairingState> {
        let config = Config::global();
        let entries: Vec<StoredPairing> = config.get_param(PAIRINGS_CONFIG_KEY).unwrap_or_default();
        let mut map = HashMap::new();
        for entry in entries {
            let user = PlatformUser {
                platform: entry.platform,
                user_id: entry.user_id,
                display_name: entry.display_name,
            };
            map.insert(user, entry.state);
        }
        map
    }

    fn save_pairings_to_config(
        pairings: &HashMap<PlatformUser, PairingState>,
    ) -> anyhow::Result<()> {
        let entries: Vec<StoredPairing> = pairings
            .iter()
            .map(|(user, state)| StoredPairing {
                platform: user.platform.clone(),
                user_id: user.user_id.clone(),
                display_name: user.display_name.clone(),
                state: state.clone(),
            })
            .collect();
        Config::global()
            .set_param(PAIRINGS_CONFIG_KEY, &entries)
            .map_err(|e| anyhow::anyhow!("failed to save gateway pairings: {}", e))
    }

    fn load_pending_codes() -> Vec<StoredPendingCode> {
        Config::global()
            .get_param(PENDING_CODES_CONFIG_KEY)
            .unwrap_or_default()
    }

    fn save_pending_codes(codes: &[StoredPendingCode]) -> anyhow::Result<()> {
        Config::global()
            .set_param(PENDING_CODES_CONFIG_KEY, codes)
            .map_err(|e| anyhow::anyhow!("failed to save pending codes: {}", e))
    }

    pub async fn get(&self, user: &PlatformUser) -> anyhow::Result<PairingState> {
        let pairings = self.pairings.read().await;
        Ok(pairings
            .get(user)
            .cloned()
            .unwrap_or(PairingState::Unpaired))
    }

    pub async fn set(&self, user: &PlatformUser, state: PairingState) -> anyhow::Result<()> {
        let mut pairings = self.pairings.write().await;
        pairings.insert(user.clone(), state);
        Self::save_pairings_to_config(&pairings)
    }

    pub async fn remove(&self, user: &PlatformUser) -> anyhow::Result<()> {
        let mut pairings = self.pairings.write().await;
        pairings.remove(user);
        Self::save_pairings_to_config(&pairings)
    }

    pub async fn store_pending_code(
        &self,
        code: &str,
        gateway_type: &str,
        expires_at: i64,
    ) -> anyhow::Result<()> {
        let mut codes = Self::load_pending_codes();
        codes.retain(|c| c.code != code);
        codes.push(StoredPendingCode {
            code: code.to_string(),
            gateway_type: gateway_type.to_string(),
            expires_at,
        });
        Self::save_pending_codes(&codes)
    }

    pub async fn consume_pending_code(&self, code: &str) -> anyhow::Result<Option<String>> {
        let mut codes = Self::load_pending_codes();
        let pos = codes.iter().position(|c| c.code == code);
        let Some(pos) = pos else {
            return Ok(None);
        };

        let entry = codes.remove(pos);
        Self::save_pending_codes(&codes)?;

        let now = chrono::Utc::now().timestamp();
        if now > entry.expires_at {
            return Ok(None);
        }

        Ok(Some(entry.gateway_type))
    }

    pub fn generate_code() -> String {
        use rand::Rng;
        let chars: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
        let mut rng = rand::thread_rng();
        (0..6)
            .map(|_| chars[rng.gen_range(0..chars.len())] as char)
            .collect()
    }

    pub async fn remove_all_for_platform(&self, platform: &str) -> anyhow::Result<usize> {
        let mut pairings = self.pairings.write().await;
        let before = pairings.len();
        pairings.retain(|user, _| user.platform != platform);
        let removed = before - pairings.len();
        Self::save_pairings_to_config(&pairings)?;
        Ok(removed)
    }

    pub async fn list_paired_users(
        &self,
        gateway_type: &str,
    ) -> anyhow::Result<Vec<(PlatformUser, String, i64)>> {
        let pairings = self.pairings.read().await;
        let mut result = Vec::new();
        for (user, state) in pairings.iter() {
            if user.platform == gateway_type {
                if let PairingState::Paired {
                    session_id,
                    paired_at,
                } = state
                {
                    result.push((user.clone(), session_id.clone(), *paired_at));
                }
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_generation() {
        let code = PairingStore::generate_code();
        assert_eq!(code.len(), 6);
        assert!(code
            .chars()
            .all(|c| "ABCDEFGHJKLMNPQRSTUVWXYZ23456789".contains(c)));
    }
}
