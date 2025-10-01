use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct RecipeSetupState {
    pub trust_granted: bool,
    pub parameters_satisfied: bool,
}