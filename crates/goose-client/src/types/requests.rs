use goose::agents::ExtensionConfig;
use goose::conversation::message::Message;
use goose::permission::permission_confirmation::{Permission, PrincipalType};
use goose::recipe::Recipe;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Serialize)]
pub struct StartAgentRequest {
    pub working_dir: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recipe: Option<Recipe>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recipe_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recipe_deeplink: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extension_overrides: Option<Vec<ExtensionConfig>>,
}

impl StartAgentRequest {
    pub fn new(working_dir: impl Into<String>) -> Self {
        Self {
            working_dir: working_dir.into(),
            recipe: None,
            recipe_id: None,
            recipe_deeplink: None,
            extension_overrides: None,
        }
    }

    pub fn with_recipe(mut self, recipe: Recipe) -> Self {
        self.recipe = Some(recipe);
        self
    }

    pub fn with_recipe_id(mut self, id: impl Into<String>) -> Self {
        self.recipe_id = Some(id.into());
        self
    }

    pub fn with_recipe_deeplink(mut self, deeplink: impl Into<String>) -> Self {
        self.recipe_deeplink = Some(deeplink.into());
        self
    }

    pub fn with_extension_overrides(mut self, overrides: Vec<ExtensionConfig>) -> Self {
        self.extension_overrides = Some(overrides);
        self
    }
}

#[derive(Debug, Serialize)]
pub struct ResumeAgentRequest {
    pub session_id: String,
    pub load_model_and_extensions: bool,
}

#[derive(Debug, Serialize)]
pub struct StopAgentRequest {
    pub session_id: String,
}

#[derive(Debug, Serialize)]
pub struct RestartAgentRequest {
    pub session_id: String,
}

#[derive(Debug, Serialize)]
pub struct UpdateWorkingDirRequest {
    pub session_id: String,
    pub working_dir: String,
}

#[derive(Debug, Serialize)]
pub struct UpdateFromSessionRequest {
    pub session_id: String,
}

#[derive(Debug, Serialize)]
pub struct UpdateProviderRequest {
    pub session_id: String,
    pub provider: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_limit: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_params: Option<HashMap<String, serde_json::Value>>,
}

impl UpdateProviderRequest {
    pub fn new(
        session_id: impl Into<String>,
        provider: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        Self {
            session_id: session_id.into(),
            provider: provider.into(),
            model: Some(model.into()),
            context_limit: None,
            request_params: None,
        }
    }

    pub fn new_without_model(session_id: impl Into<String>, provider: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            provider: provider.into(),
            model: None,
            context_limit: None,
            request_params: None,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct AddExtensionRequest {
    pub session_id: String,
    pub config: ExtensionConfig,
}

#[derive(Debug, Serialize)]
pub struct RemoveExtensionRequest {
    pub name: String,
    pub session_id: String,
}

#[derive(Debug, Serialize)]
pub struct ChatRequest {
    pub user_message: Message,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub override_conversation: Option<Vec<Message>>,
    pub session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recipe_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recipe_version: Option<String>,
}

impl ChatRequest {
    pub fn new(session_id: impl Into<String>, user_message: Message) -> Self {
        Self {
            user_message,
            override_conversation: None,
            session_id: session_id.into(),
            recipe_name: None,
            recipe_version: None,
        }
    }

    pub fn with_override_conversation(mut self, conversation: Vec<Message>) -> Self {
        self.override_conversation = Some(conversation);
        self
    }
}

/// Tool confirmation request. All fields are camelCase to match goose-server's
/// `ConfirmToolActionRequest` which uses `#[serde(rename_all = "camelCase")]`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmToolActionRequest {
    pub id: String,
    pub principal_type: PrincipalType,
    pub action: Permission,
    pub session_id: String,
}

impl ConfirmToolActionRequest {
    pub fn new(id: impl Into<String>, action: Permission, session_id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            principal_type: PrincipalType::Tool,
            action,
            session_id: session_id.into(),
        }
    }

    pub fn with_principal_type(mut self, principal_type: PrincipalType) -> Self {
        self.principal_type = principal_type;
        self
    }
}

/// Session rename request. camelCase to match server.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSessionNameRequest {
    pub name: String,
}

/// Session import request. camelCase to match server.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSessionRequest {
    pub json: String,
}

/// Session fork request. camelCase to match server.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForkRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<i64>,
    pub truncate: bool,
    pub copy: bool,
}

#[derive(Debug, Serialize)]
pub struct CallToolRequest {
    pub session_id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct ReadResourceRequest {
    pub session_id: String,
    pub extension_name: String,
    pub uri: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportAppRequest {
    pub html: String,
}

#[derive(Debug, Serialize)]
pub struct SetContainerRequest {
    pub session_id: String,
    pub container_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserRecipeValuesRequest {
    pub user_recipe_values: HashMap<String, String>,
}
