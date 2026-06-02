use crate::config::GooseMode;
use crate::model::ModelConfig;
use crate::recipe::Recipe;
use crate::session::extension_data::ExtensionData;
use crate::session::model::SessionType;
use crate::session::session_manager::SessionManager;
use anyhow::Result;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::path::PathBuf;

pub struct SessionUpdateBuilder<'a> {
    pub(crate) session_manager: &'a SessionManager,
    pub(crate) session_id: String,
    pub(crate) name: Option<String>,
    pub(crate) user_set_name: Option<bool>,
    pub(crate) session_type: Option<SessionType>,
    pub(crate) working_dir: Option<PathBuf>,
    pub(crate) extension_data: Option<ExtensionData>,
    pub(crate) total_tokens: Option<Option<i32>>,
    pub(crate) input_tokens: Option<Option<i32>>,
    pub(crate) output_tokens: Option<Option<i32>>,
    pub(crate) accumulated_total_tokens: Option<Option<i32>>,
    pub(crate) accumulated_input_tokens: Option<Option<i32>>,
    pub(crate) accumulated_output_tokens: Option<Option<i32>>,
    pub(crate) accumulated_cost: Option<Option<f64>>,
    pub(crate) schedule_id: Option<Option<String>>,
    pub(crate) recipe: Option<Option<Recipe>>,
    pub(crate) user_recipe_values: Option<Option<HashMap<String, String>>>,
    pub(crate) provider_name: Option<Option<String>>,
    pub(crate) model_config: Option<Option<ModelConfig>>,
    pub(crate) goose_mode: Option<GooseMode>,
    pub(crate) archived_at: Option<Option<DateTime<Utc>>>,
    pub(crate) project_id: Option<Option<String>>,
}

impl<'a> SessionUpdateBuilder<'a> {
    pub(crate) fn new(session_manager: &'a SessionManager, session_id: String) -> Self {
        Self {
            session_manager,
            session_id,
            name: None,
            user_set_name: None,
            session_type: None,
            working_dir: None,
            extension_data: None,
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
            provider_name: None,
            model_config: None,
            goose_mode: None,
            archived_at: None,
            project_id: None,
        }
    }

    pub async fn apply(self) -> Result<()> {
        self.session_manager.apply_update_inner(self).await
    }

    pub fn user_provided_name(mut self, name: impl Into<String>) -> Self {
        let name = name.into().trim().to_string();
        if !name.is_empty() {
            self.name = Some(name);
            self.user_set_name = Some(true);
        }
        self
    }

    pub fn system_generated_name(mut self, name: impl Into<String>) -> Self {
        let name = name.into().trim().to_string();
        if !name.is_empty() {
            self.name = Some(name);
            self.user_set_name = Some(false);
        }
        self
    }

    pub fn session_type(mut self, session_type: SessionType) -> Self {
        self.session_type = Some(session_type);
        self
    }

    pub fn working_dir(mut self, working_dir: PathBuf) -> Self {
        self.working_dir = Some(working_dir);
        self
    }

    pub fn extension_data(mut self, data: ExtensionData) -> Self {
        self.extension_data = Some(data);
        self
    }

    pub fn total_tokens(mut self, tokens: Option<i32>) -> Self {
        self.total_tokens = Some(tokens);
        self
    }

    pub fn input_tokens(mut self, tokens: Option<i32>) -> Self {
        self.input_tokens = Some(tokens);
        self
    }

    pub fn output_tokens(mut self, tokens: Option<i32>) -> Self {
        self.output_tokens = Some(tokens);
        self
    }

    pub fn accumulated_total_tokens(mut self, tokens: Option<i32>) -> Self {
        self.accumulated_total_tokens = Some(tokens);
        self
    }

    pub fn accumulated_input_tokens(mut self, tokens: Option<i32>) -> Self {
        self.accumulated_input_tokens = Some(tokens);
        self
    }

    pub fn accumulated_output_tokens(mut self, tokens: Option<i32>) -> Self {
        self.accumulated_output_tokens = Some(tokens);
        self
    }

    pub fn accumulated_cost(mut self, cost: Option<f64>) -> Self {
        self.accumulated_cost = Some(cost);
        self
    }

    pub fn schedule_id(mut self, schedule_id: Option<String>) -> Self {
        self.schedule_id = Some(schedule_id);
        self
    }

    pub fn recipe(mut self, recipe: Option<Recipe>) -> Self {
        self.recipe = Some(recipe);
        self
    }

    pub fn user_recipe_values(
        mut self,
        user_recipe_values: Option<HashMap<String, String>>,
    ) -> Self {
        self.user_recipe_values = Some(user_recipe_values);
        self
    }

    pub fn provider_name(mut self, provider_name: impl Into<String>) -> Self {
        self.provider_name = Some(Some(provider_name.into()));
        self
    }

    pub fn model_config(mut self, model_config: ModelConfig) -> Self {
        self.model_config = Some(Some(model_config));
        self
    }

    pub fn clear_model_config(mut self) -> Self {
        self.model_config = Some(None);
        self
    }

    pub fn goose_mode(mut self, mode: GooseMode) -> Self {
        self.goose_mode = Some(mode);
        self
    }

    pub fn archived_at(mut self, archived_at: Option<DateTime<Utc>>) -> Self {
        self.archived_at = Some(archived_at);
        self
    }

    pub fn project_id(mut self, project_id: Option<String>) -> Self {
        self.project_id = Some(project_id);
        self
    }
}
