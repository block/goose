use std::collections::HashMap;

use agent_client_protocol::{JsonRpcRequest, JsonRpcResponse};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::EmptyResponse;

fn default_recipe_version() -> String {
    "1.0.0".to_string()
}

pub const REQUEST_RECIPE_PARAMS_METHOD: &str = "_goose/unstable/session/recipe/request-params";

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RecipeDto {
    #[serde(default = "default_recipe_version")]
    pub version: String,
    pub title: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Vec<RecipeExtensionDto>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings: Option<RecipeSettingsDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activities: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<RecipeAuthorDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Vec<RecipeParameterDto>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response: Option<RecipeResponseDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sub_recipes: Option<Vec<SubRecipeDto>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry: Option<RecipeRetryConfigDto>,
}

impl Default for RecipeDto {
    fn default() -> Self {
        Self {
            version: default_recipe_version(),
            title: String::new(),
            description: String::new(),
            instructions: None,
            prompt: None,
            extensions: None,
            settings: None,
            activities: None,
            author: None,
            parameters: None,
            response: None,
            sub_recipes: None,
            retry: None,
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RecipeAuthorDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contact: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RecipeSettingsDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub goose_provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub goose_model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_turns: Option<usize>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RecipeResponseDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub json_schema: Option<serde_json::Value>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SubRecipeDto {
    pub name: String,
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub values: Option<HashMap<String, String>>,
    #[serde(default)]
    pub sequential_when_repeated: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RecipeParameterDto {
    pub key: String,
    pub input_type: RecipeParameterInputTypeDto,
    pub requirement: RecipeParameterRequirementDto,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<String>>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RecipeParameterInputTypeDto {
    #[default]
    String,
    Number,
    Boolean,
    Date,
    File,
    Select,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RecipeParameterRequirementDto {
    #[default]
    Required,
    Optional,
    UserPrompt,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RecipeRetryConfigDto {
    pub max_retries: u32,
    #[serde(default)]
    pub checks: Vec<RecipeSuccessCheckDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_failure: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_seconds: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_failure_timeout_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RecipeSuccessCheckDto {
    Shell { command: String },
}

impl Default for RecipeSuccessCheckDto {
    fn default() -> Self {
        Self::Shell {
            command: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RecipeExtensionDto {
    Builtin {
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        display_name: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        timeout: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bundled: Option<bool>,
    },
    Platform {
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        display_name: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bundled: Option<bool>,
    },
    Stdio {
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        cmd: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        args: Vec<String>,
        #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
        envs: HashMap<String, String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        env_keys: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        timeout: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cwd: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bundled: Option<bool>,
    },
    StreamableHttp {
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        uri: String,
        #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
        envs: HashMap<String, String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        env_keys: Vec<String>,
        #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
        headers: HashMap<String, String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        timeout: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        socket: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bundled: Option<bool>,
    },
}

impl Default for RecipeExtensionDto {
    fn default() -> Self {
        Self::Builtin {
            name: String::new(),
            description: None,
            display_name: None,
            timeout: None,
            bundled: None,
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RecipeListEntryDto {
    pub id: String,
    pub recipe: RecipeDto,
    pub file_path: String,
    pub last_modified: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schedule_cron: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slash_command: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RequestRecipeParams {
    pub session_id: String,
    pub parameters: Vec<RecipeParameterDto>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum RecipeParamsAction {
    #[default]
    Submit,
    Cancel,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RecipeParamsResponse {
    #[serde(default)]
    pub action: RecipeParamsAction,
    #[serde(default)]
    pub values: HashMap<String, String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(
    method = "_goose/unstable/recipes/encode",
    response = EncodeRecipeResponse
)]
pub struct EncodeRecipeRequest {
    pub recipe: RecipeDto,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcResponse)]
pub struct EncodeRecipeResponse {
    pub deeplink: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(
    method = "_goose/unstable/recipes/decode",
    response = DecodeRecipeResponse
)]
pub struct DecodeRecipeRequest {
    pub deeplink: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcResponse)]
pub struct DecodeRecipeResponse {
    pub recipe: RecipeDto,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(method = "_goose/unstable/recipes/scan", response = ScanRecipeResponse)]
pub struct ScanRecipeRequest {
    pub recipe: RecipeDto,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcResponse)]
pub struct ScanRecipeResponse {
    pub has_security_warnings: bool,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(method = "_goose/unstable/recipes/save", response = SaveRecipeResponse)]
pub struct SaveRecipeRequest {
    pub recipe: RecipeDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcResponse)]
pub struct SaveRecipeResponse {
    pub id: String,
    pub file_name: String,
    pub file_path: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(method = "_goose/unstable/recipes/parse", response = ParseRecipeResponse)]
pub struct ParseRecipeRequest {
    pub content: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcResponse)]
pub struct ParseRecipeResponse {
    pub recipe: RecipeDto,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(method = "_goose/unstable/recipes/delete", response = EmptyResponse)]
pub struct DeleteRecipeRequest {
    pub id: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(method = "_goose/unstable/recipes/list", response = ListRecipesResponse)]
pub struct ListRecipesRequest {}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcResponse)]
pub struct ListRecipesResponse {
    pub recipes: Vec<RecipeListEntryDto>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(method = "_goose/unstable/recipes/schedule", response = EmptyResponse)]
pub struct ScheduleRecipeRequest {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cron_schedule: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(
    method = "_goose/unstable/recipes/slash-command",
    response = EmptyResponse
)]
pub struct SetRecipeSlashCommandRequest {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slash_command: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcRequest)]
#[request(
    method = "_goose/unstable/recipes/to-yaml",
    response = RecipeToYamlResponse
)]
pub struct RecipeToYamlRequest {
    pub recipe: RecipeDto,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, JsonRpcResponse)]
pub struct RecipeToYamlResponse {
    pub yaml: String,
}
