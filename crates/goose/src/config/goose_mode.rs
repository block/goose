use serde::{Deserialize, Serialize};
use strum::{Display, EnumString, IntoStaticStr, VariantNames};

#[derive(
    Copy,
    Clone,
    Debug,
    Eq,
    PartialEq,
    Serialize,
    Deserialize,
    Display,
    EnumString,
    IntoStaticStr,
    VariantNames,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum GooseMode {
    Auto,
    Approve,
    SmartApprove,
    Chat,
}
