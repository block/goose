use crate::config::GooseMode;
use crate::conversation::Conversation;
use crate::model::ModelConfig;
use crate::recipe::Recipe;
use crate::session::extension_data::ExtensionData;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use utoipa::ToSchema;

#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    ToSchema,
    PartialEq,
    Eq,
    Default,
    strum::Display,
    strum::EnumString,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum SessionType {
    #[default]
    User,
    Scheduled,
    SubAgent,
    Hidden,
    Terminal,
    Gateway,
    Acp,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Session {
    pub id: String,
    #[schema(value_type = String)]
    pub working_dir: PathBuf,
    #[serde(alias = "description")]
    pub name: String,
    #[serde(default)]
    pub user_set_name: bool,
    #[serde(default)]
    pub session_type: SessionType,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub extension_data: ExtensionData,
    pub total_tokens: Option<i32>,
    pub input_tokens: Option<i32>,
    pub output_tokens: Option<i32>,
    pub accumulated_total_tokens: Option<i32>,
    pub accumulated_input_tokens: Option<i32>,
    pub accumulated_output_tokens: Option<i32>,
    pub accumulated_cost: Option<f64>,
    pub schedule_id: Option<String>,
    pub recipe: Option<Recipe>,
    pub user_recipe_values: Option<HashMap<String, String>>,
    pub conversation: Option<Conversation>,
    pub message_count: usize,
    pub provider_name: Option<String>,
    pub model_config: Option<ModelConfig>,
    #[serde(default)]
    pub goose_mode: GooseMode,
    #[serde(default)]
    pub archived_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub project_id: Option<String>,
}

#[derive(Serialize, ToSchema, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SessionInsights {
    pub total_sessions: usize,
    pub total_tokens: i64,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            id: String::new(),
            working_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            name: String::new(),
            user_set_name: false,
            session_type: SessionType::default(),
            created_at: Default::default(),
            updated_at: Default::default(),
            extension_data: ExtensionData::default(),
            total_tokens: None,
            input_tokens: None,
            output_tokens: None,
            accumulated_total_tokens: None,
            accumulated_input_tokens: None,
            accumulated_output_tokens: None,
            accumulated_cost: None,
            schedule_id: None,
            recipe: None,
            user_recipe_values: None,
            conversation: None,
            message_count: 0,
            provider_name: None,
            model_config: None,
            goose_mode: GooseMode::default(),
            archived_at: None,
            project_id: None,
        }
    }
}

impl Session {
    pub fn without_messages(mut self) -> Self {
        self.conversation = None;
        self
    }
}

impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for Session {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;

        let recipe_json: Option<String> = row.try_get("recipe_json")?;
        let recipe = recipe_json.and_then(|json| serde_json::from_str(&json).ok());

        let user_recipe_values_json: Option<String> = row.try_get("user_recipe_values_json")?;
        let user_recipe_values =
            user_recipe_values_json.and_then(|json| serde_json::from_str(&json).ok());

        let model_config_json: Option<String> = row.try_get("model_config_json").ok().flatten();
        let model_config = model_config_json.and_then(|json| serde_json::from_str(&json).ok());

        let name: String = {
            let name_val: String = row.try_get("name").unwrap_or_default();
            if !name_val.is_empty() {
                name_val
            } else {
                row.try_get("description").unwrap_or_default()
            }
        };

        let user_set_name = row.try_get("user_set_name").unwrap_or(false);

        let session_type_str: String = row
            .try_get("session_type")
            .unwrap_or_else(|_| "user".to_string());
        let session_type = session_type_str.parse().unwrap_or_default();

        Ok(Session {
            id: row.try_get("id")?,
            working_dir: PathBuf::from(row.try_get::<String, _>("working_dir")?),
            name,
            user_set_name,
            session_type,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
            extension_data: serde_json::from_str(&row.try_get::<String, _>("extension_data")?)
                .unwrap_or_default(),
            total_tokens: row.try_get("total_tokens")?,
            input_tokens: row.try_get("input_tokens")?,
            output_tokens: row.try_get("output_tokens")?,
            accumulated_total_tokens: row.try_get("accumulated_total_tokens")?,
            accumulated_input_tokens: row.try_get("accumulated_input_tokens")?,
            accumulated_output_tokens: row.try_get("accumulated_output_tokens")?,
            accumulated_cost: row.try_get("accumulated_cost").ok().flatten(),
            schedule_id: row.try_get("schedule_id")?,
            recipe,
            user_recipe_values,
            conversation: None,
            message_count: row.try_get("message_count").unwrap_or(0) as usize,
            provider_name: row.try_get("provider_name").ok().flatten(),
            model_config,
            goose_mode: row
                .try_get::<String, _>("goose_mode")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or_default(),
            archived_at: row.try_get("archived_at").ok(),
            project_id: row.try_get("project_id").ok().flatten(),
        })
    }
}
