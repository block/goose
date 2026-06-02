use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInventoryEntry {
    pub provider_id: String,
    pub provider_name: String,
    pub description: String,
    pub default_model: String,
    pub configured: bool,
    pub provider_type: ProviderType,
    pub category: ProviderSetupCategory,
    pub config_keys: Vec<ConfigKey>,
    pub setup_steps: Vec<String>,
    pub supports_refresh: bool,
    pub refreshing: bool,
    pub models: Vec<InventoryModel>,
    pub last_updated_at: Option<DateTime<Utc>>,
    pub last_refresh_attempt_at: Option<DateTime<Utc>>,
    pub last_refresh_error: Option<String>,
    pub model_selection_hint: Option<String>,
}

/// Families whose latest model should be surfaced in the compact picker.
/// Each entry is matched against the `family` field of enriched models.
pub const RECOMMENDED_FAMILIES: &[&str] = &[
    "claude-opus",
    "claude-sonnet",
    "gpt",
    "gpt-mini",
    "glm",
    "gemini-pro",
    "gemini-flash",
    "gemma",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryModel {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_limit: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<bool>,
    /// Whether this model should appear in the compact recommended picker.
    pub recommended: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventoryIdentity {
    pub provider_id: String,
    pub provider_family: String,
    pub inventory_key: String,
}

#[derive(Debug, Clone, Default)]
pub struct InventoryIdentityInput {
    pub provider_id: String,
    pub provider_family: String,
    pub public_inputs: BTreeMap<String, String>,
    pub secret_inputs: BTreeMap<String, String>,
}

impl InventoryIdentityInput {
    pub fn new(
        provider_id: impl Into<String>,
        provider_family: impl Into<String>,
    ) -> InventoryIdentityInput {
        InventoryIdentityInput {
            provider_id: provider_id.into(),
            provider_family: provider_family.into(),
            public_inputs: BTreeMap::new(),
            secret_inputs: BTreeMap::new(),
        }
    }

    pub fn with_public(
        mut self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> InventoryIdentityInput {
        self.public_inputs.insert(key.into(), value.into());
        self
    }

    pub fn with_secret(
        mut self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> InventoryIdentityInput {
        self.secret_inputs.insert(key.into(), value.into());
        self
    }

    pub fn into_identity(self) -> Result<InventoryIdentity> {
        let InventoryIdentityInput {
            provider_id,
            provider_family,
            public_inputs,
            secret_inputs,
        } = self;
        let payload = serde_json::json!({
            "provider_family": provider_family,
            "public_inputs": public_inputs,
            "secret_inputs": secret_inputs,
        });
        let digest = Sha256::digest(serde_json::to_vec(&payload)?);
        Ok(InventoryIdentity {
            provider_id,
            provider_family,
            inventory_key: format!("{digest:x}"),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefreshSkipReason {
    UnknownProvider,
    NotConfigured,
    DoesNotSupportRefresh,
    AlreadyRefreshing,
}

#[derive(Debug, Clone)]
pub struct RefreshSkip {
    pub provider_id: String,
    pub reason: RefreshSkipReason,
}

#[derive(Debug, Clone)]
pub(crate) struct RefreshJob {
    pub provider_id: String,
    pub identity: InventoryIdentity,
}

#[derive(Debug, Clone, Default)]
pub struct RefreshPlan {
    pub started: Vec<String>,
    pub skipped: Vec<RefreshSkip>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct RefreshJobPlan {
    pub started: Vec<RefreshJob>,
    pub skipped: Vec<RefreshSkip>,
}

impl RefreshJobPlan {
    pub(crate) fn into_public_plan(self) -> RefreshPlan {
        RefreshPlan {
            started: self
                .started
                .into_iter()
                .map(|job| job.provider_id)
                .collect(),
            skipped: self.skipped,
        }
    }
}
