use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use agent_client_protocol::schema::Meta;
use agent_client_protocol::{
    Client, ConnectionTo, JsonRpcMessage, JsonRpcRequest, JsonRpcResponse, UntypedMessage,
};
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;

use super::{meta_string, GooseAcpAgent, ResultExt};
use crate::agents::Agent;
use crate::recipe::build_recipe::{build_recipe_from_template, RecipeError};
use crate::recipe::local_recipes::get_recipe_library_dir;
use crate::recipe::manifest::{list_recipe_file_manifests, load_recipe_from_path};
use crate::recipe::validate_recipe::validate_recipe_template_from_content;
use crate::recipe::{Recipe, RecipeParameter};
use crate::recipe_deeplink;

pub(super) const RECIPE_PARAMS_METHOD: &str = "_goose/unstable/session/recipe/request-params";

impl GooseAcpAgent {
    pub(super) async fn resolve_recipe_from_meta(
        &self,
        meta: Option<&Meta>,
    ) -> Result<Option<Recipe>, agent_client_protocol::Error> {
        let recipe = if let Some(deeplink) = meta_string(meta, "recipeDeeplink")? {
            Some(recipe_deeplink::decode(&deeplink).map_err(|e| {
                agent_client_protocol::Error::invalid_params().data(format!("recipeDeeplink: {e}"))
            })?)
        } else if let Some(id) = meta_string(meta, "recipeId")? {
            let path = self.resolve_recipe_path_by_id(&id).await?;
            Some(load_recipe_from_path(&path).internal_err_ctx("Failed to load recipe")?)
        } else {
            None
        };

        if let Some(ref recipe) = recipe {
            validate_recipe(recipe)?;
        }
        Ok(recipe)
    }

    async fn resolve_recipe_path_by_id(
        &self,
        id: &str,
    ) -> Result<PathBuf, agent_client_protocol::Error> {
        if let Some(path) = self.recipe_path_cache.lock().await.get(id).cloned() {
            return Ok(path);
        }
        let map: HashMap<String, PathBuf> = list_recipe_file_manifests()
            .unwrap_or_default()
            .into_iter()
            .map(|manifest| (manifest.id, manifest.file_path))
            .collect();
        let resolved = map.get(id).cloned();
        *self.recipe_path_cache.lock().await = map;
        resolved.ok_or_else(|| {
            agent_client_protocol::Error::invalid_params().data(format!("recipe not found: {id}"))
        })
    }

    pub(super) fn render_recipe(
        &self,
        recipe: &Recipe,
        values: HashMap<String, String>,
    ) -> Result<Option<Recipe>, agent_client_protocol::Error> {
        let content = recipe.to_yaml().map_err(|e| {
            agent_client_protocol::Error::invalid_params().data(format!("recipe: {e}"))
        })?;
        let recipe_dir = get_recipe_library_dir(true);
        let params: Vec<(String, String)> = values.into_iter().collect();
        match build_recipe_from_template(
            content,
            &recipe_dir,
            params,
            None::<fn(&str, &str) -> Result<String, anyhow::Error>>,
        ) {
            Ok(rendered) => Ok(Some(rendered)),
            Err(RecipeError::MissingParams { .. }) => Ok(None),
            Err(e) => {
                Err(agent_client_protocol::Error::internal_error().data(format!("recipe: {e}")))
            }
        }
    }

    pub(super) async fn apply_recipe(&self, agent: &Arc<Agent>, recipe: &Recipe) {
        agent
            .apply_recipe_components(recipe.response.clone(), true)
            .await;
        if let Some(instructions) = recipe.instructions.clone() {
            agent
                .extend_system_prompt("recipe".to_string(), instructions)
                .await;
        }
    }

    pub(super) async fn render_recipe_or_cleanup(
        &self,
        cx: &ConnectionTo<Client>,
        session_id: &str,
        recipe: Option<&Recipe>,
    ) -> Result<(Option<Recipe>, Option<HashMap<String, String>>), agent_client_protocol::Error>
    {
        let Some(recipe) = recipe else {
            return Ok((None, None));
        };
        match self.render_recipe_with_params(cx, session_id, recipe).await {
            Ok((rendered, values)) => Ok((Some(rendered), values)),
            Err(e) => {
                let _ = self.session_manager.delete_session(session_id).await;
                Err(e)
            }
        }
    }

    pub(super) async fn render_recipe_with_params(
        &self,
        cx: &ConnectionTo<Client>,
        session_id: &str,
        recipe: &Recipe,
    ) -> Result<(Recipe, Option<HashMap<String, String>>), agent_client_protocol::Error> {
        if let Some(rendered) = self.render_recipe(recipe, HashMap::new())? {
            return Ok((rendered, None));
        }
        if !self.supports_recipe_param_requests() {
            return Err(agent_client_protocol::Error::invalid_params().data(
                "recipe requires parameters but the client does not support recipeParameterRequests",
            ));
        }
        let parameters = recipe.parameters.clone().unwrap_or_default();
        let values = self
            .request_recipe_params(cx, session_id, parameters)
            .await?;
        match self.render_recipe(recipe, values.clone())? {
            Some(rendered) => Ok((rendered, Some(values))),
            None => Err(agent_client_protocol::Error::invalid_params()
                .data("recipe still missing required parameters")),
        }
    }

    pub(super) async fn request_recipe_params(
        &self,
        cx: &ConnectionTo<Client>,
        session_id: &str,
        parameters: Vec<RecipeParameter>,
    ) -> Result<HashMap<String, String>, agent_client_protocol::Error> {
        let request = RequestRecipeParams {
            session_id: session_id.to_string(),
            parameters,
        };
        let (tx, rx) = oneshot::channel();
        cx.send_request(RequestRecipeParamsMessage(request))
            .on_receiving_result(move |result| async move {
                let _ = tx.send(result.map(|response| response.0.values));
                Ok(())
            })?;
        match rx.await {
            Ok(values) => values,
            Err(_) => Err(agent_client_protocol::Error::internal_error()
                .data("recipe params request was dropped")),
        }
    }
}

fn validate_recipe(recipe: &Recipe) -> Result<(), agent_client_protocol::Error> {
    let yaml = recipe
        .to_yaml()
        .map_err(|e| agent_client_protocol::Error::invalid_params().data(format!("recipe: {e}")))?;
    validate_recipe_template_from_content(&yaml, None)
        .map_err(|e| agent_client_protocol::Error::invalid_params().data(format!("recipe: {e}")))?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct RequestRecipeParams {
    session_id: String,
    parameters: Vec<RecipeParameter>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct RecipeParamsResponse {
    #[serde(default)]
    values: HashMap<String, String>,
}

#[derive(Debug, Clone)]
struct RequestRecipeParamsMessage(RequestRecipeParams);

impl JsonRpcMessage for RequestRecipeParamsMessage {
    fn matches_method(method: &str) -> bool {
        method == RECIPE_PARAMS_METHOD
    }

    fn method(&self) -> &str {
        RECIPE_PARAMS_METHOD
    }

    fn to_untyped_message(&self) -> Result<UntypedMessage, agent_client_protocol::Error> {
        UntypedMessage::new(RECIPE_PARAMS_METHOD, &self.0)
    }

    fn parse_message(
        method: &str,
        params: &impl serde::Serialize,
    ) -> Result<Self, agent_client_protocol::Error> {
        if !Self::matches_method(method) {
            return Err(agent_client_protocol::Error::method_not_found());
        }
        Ok(Self(agent_client_protocol::util::json_cast_params(params)?))
    }
}

impl JsonRpcRequest for RequestRecipeParamsMessage {
    type Response = RecipeParamsResponseMessage;
}

#[derive(Debug, Clone)]
struct RecipeParamsResponseMessage(RecipeParamsResponse);

impl JsonRpcResponse for RecipeParamsResponseMessage {
    fn into_json(self, _method: &str) -> Result<serde_json::Value, agent_client_protocol::Error> {
        serde_json::to_value(self.0).map_err(agent_client_protocol::Error::into_internal_error)
    }

    fn from_value(
        _method: &str,
        value: serde_json::Value,
    ) -> Result<Self, agent_client_protocol::Error> {
        Ok(Self(agent_client_protocol::util::json_cast(&value)?))
    }
}
