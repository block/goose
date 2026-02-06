use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    AlwaysAllow,
    AllowOnce,
    Cancel,
    DenyOnce,
    AlwaysDeny,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, ToSchema)]
pub enum PrincipalType {
    Extension,
    Tool,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct PermissionConfirmation {
    pub principal_type: PrincipalType,
    pub permission: Permission,
}
