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
    #[serde(default)]
    session_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingPairing {
    pub gateway_type: String,
    pub session_id: Option<String>,
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

    pub async fn pair_with_session(
        &self,
        user: &PlatformUser,
        session_id: &str,
        paired_at: i64,
        exclusive_session: bool,
    ) -> anyhow::Result<()> {
        let mut pairings = self.pairings.write().await;
        if exclusive_session && session_has_other_pairing(&pairings, user, session_id) {
            anyhow::bail!(
                "session '{}' is already paired to another {} group",
                session_id,
                user.platform
            );
        }

        pairings.insert(
            user.clone(),
            PairingState::Paired {
                session_id: session_id.to_string(),
                paired_at,
            },
        );
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
        session_id: Option<&str>,
    ) -> anyhow::Result<()> {
        let mut codes = Self::load_pending_codes();
        codes.retain(|c| c.code != code);
        codes.push(StoredPendingCode {
            code: code.to_string(),
            gateway_type: gateway_type.to_string(),
            expires_at,
            session_id: session_id.map(str::to_string),
        });
        Self::save_pending_codes(&codes)
    }

    pub async fn consume_pending_code(&self, code: &str) -> anyhow::Result<Option<PendingPairing>> {
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

        Ok(Some(PendingPairing {
            gateway_type: entry.gateway_type,
            session_id: entry.session_id,
        }))
    }

    pub fn generate_code() -> String {
        use rand::RngExt;
        let chars: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
        let mut rng = rand::rng();
        (0..6)
            .map(|_| chars[rng.random_range(0..chars.len())] as char)
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

fn session_has_other_pairing(
    pairings: &HashMap<PlatformUser, PairingState>,
    user: &PlatformUser,
    session_id: &str,
) -> bool {
    pairings.iter().any(|(paired_user, state)| {
        paired_user != user
            && paired_user.platform == user.platform
            && matches!(
                state,
                PairingState::Paired {
                    session_id: paired_session,
                    ..
                } if paired_session == session_id
            )
    })
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

    #[test]
    fn legacy_pending_codes_deserialize_without_session() {
        let code: StoredPendingCode = serde_json::from_value(serde_json::json!({
            "code": "ABC234",
            "gateway_type": "telegram",
            "expires_at": 42
        }))
        .unwrap();

        assert_eq!(code.session_id, None);
    }

    #[test]
    fn sonar_session_pairing_is_exclusive_between_groups() {
        let first = PlatformUser {
            platform: "sonar".into(),
            user_id: "group-a".into(),
            display_name: None,
        };
        let second = PlatformUser {
            platform: "sonar".into(),
            user_id: "group-b".into(),
            display_name: None,
        };
        let pairings = HashMap::from([(
            first.clone(),
            PairingState::Paired {
                session_id: "session".into(),
                paired_at: 1,
            },
        )]);

        assert!(!session_has_other_pairing(&pairings, &first, "session"));
        assert!(session_has_other_pairing(&pairings, &second, "session"));
    }
}
