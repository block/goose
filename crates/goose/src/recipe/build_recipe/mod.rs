use crate::recipe::read_recipe_file_content::RecipeFile;
use crate::recipe::template_recipe::{parse_recipe_content, render_recipe_content_with_params};
use crate::recipe::{
    Recipe, RecipeParameter, RecipeParameterRequirement, BUILT_IN_RECIPE_DIR_PARAM,
};
use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum RecipeError {
    #[error("Missing required parameters: {parameters:?}")]
    MissingParams { parameters: Vec<String> },
    #[error("Template rendering failed: {source}")]
    TemplateRendering { source: anyhow::Error },
    #[error("Recipe parsing failed: {source}")]
    RecipeParsing { source: anyhow::Error },
}

pub fn render_recipe_template<F>(
    recipe_file: RecipeFile,
    params: Vec<(String, String)>,
    user_prompt_fn: Option<F>,
) -> Result<(String, Vec<String>)>
where
    F: Fn(&str, &str) -> Result<String, anyhow::Error>,
{
    let RecipeFile {
        content: recipe_file_content,
        parent_dir: recipe_parent_dir,
        ..
    } = recipe_file;
    let recipe_dir_str = recipe_parent_dir
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Error getting recipe directory"))?;
    let recipe_parameters = validate_recipe_parameters(&recipe_file_content, recipe_dir_str)?;

    let (params_for_template, missing_params) =
        apply_values_to_parameters(&params, recipe_parameters, recipe_dir_str, user_prompt_fn)?;

    let rendered_content = if missing_params.is_empty() {
        render_recipe_content_with_params(&recipe_file_content, &params_for_template)?
    } else {
        String::new()
    };

    Ok((rendered_content, missing_params))
}

pub fn validate_recipe_parameters(
    recipe_file_content: &str,
    recipe_dir_str: &str,
) -> Result<Option<Vec<RecipeParameter>>> {
    let (raw_recipe, template_variables) =
        parse_recipe_content(recipe_file_content, recipe_dir_str.to_string())?;
    let recipe_parameters = raw_recipe.parameters;
    validate_optional_parameters(&recipe_parameters)?;
    validate_parameters_in_template(&recipe_parameters, &template_variables)?;
    Ok(recipe_parameters)
}

pub fn build_recipe_from_template<F>(
    recipe_file: RecipeFile,
    params: Vec<(String, String)>,
    user_prompt_fn: Option<F>,
) -> Result<Recipe, RecipeError>
where
    F: Fn(&str, &str) -> Result<String, anyhow::Error>,
{
    let recipe_parent_dir = recipe_file.parent_dir.clone();
    let (rendered_content, missing_params) =
        render_recipe_template(recipe_file, params.clone(), user_prompt_fn)
            .map_err(|source| RecipeError::TemplateRendering { source })?;

    if !missing_params.is_empty() {
        return Err(RecipeError::MissingParams {
            parameters: missing_params,
        });
    }

    let mut recipe = Recipe::from_content(&rendered_content)
        .map_err(|source| RecipeError::RecipeParsing { source })?;

    if let Some(ref mut sub_recipes) = recipe.sub_recipes {
        for sub_recipe in sub_recipes {
            if let Ok(resolved_path) = resolve_sub_recipe_path(&sub_recipe.path, &recipe_parent_dir)
            {
                sub_recipe.path = resolved_path;
            }
        }
    }

    Ok(recipe)
}

fn validate_parameters_in_template(
    recipe_parameters: &Option<Vec<RecipeParameter>>,
    template_variables: &HashSet<String>,
) -> Result<()> {
    let mut template_variables = template_variables.clone();
    template_variables.remove(BUILT_IN_RECIPE_DIR_PARAM);

    let param_keys: HashSet<String> = recipe_parameters
        .as_ref()
        .unwrap_or(&vec![])
        .iter()
        .map(|p| p.key.clone())
        .collect();

    let missing_keys = template_variables
        .difference(&param_keys)
        .collect::<Vec<_>>();

    let extra_keys = param_keys
        .difference(&template_variables)
        .collect::<Vec<_>>();

    if missing_keys.is_empty() && extra_keys.is_empty() {
        return Ok(());
    }

    let mut message = String::new();

    if !missing_keys.is_empty() {
        message.push_str(&format!(
            "Missing definitions for parameters in the recipe file: {}.",
            missing_keys
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    if !extra_keys.is_empty() {
        message.push_str(&format!(
            "\nUnnecessary parameter definitions: {}.",
            extra_keys
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    Err(anyhow::anyhow!("{}", message.trim_end()))
}

fn validate_optional_parameters(parameters: &Option<Vec<RecipeParameter>>) -> Result<()> {
    let optional_params_without_default_values: Vec<String> = parameters
        .as_ref()
        .unwrap_or(&vec![])
        .iter()
        .filter(|p| {
            matches!(p.requirement, RecipeParameterRequirement::Optional) && p.default.is_none()
        })
        .map(|p| p.key.clone())
        .collect();

    if optional_params_without_default_values.is_empty() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("Optional parameters missing default values in the recipe: {}. Please provide defaults.", optional_params_without_default_values.join(", ")))
    }
}

pub fn apply_values_to_parameters<F>(
    user_params: &[(String, String)],
    recipe_parameters: Option<Vec<RecipeParameter>>,
    recipe_parent_dir: &str,
    user_prompt_fn: Option<F>,
) -> Result<(HashMap<String, String>, Vec<String>)>
where
    F: Fn(&str, &str) -> Result<String, anyhow::Error>,
{
    let mut param_map: HashMap<String, String> = user_params.iter().cloned().collect();
    param_map.insert(
        BUILT_IN_RECIPE_DIR_PARAM.to_string(),
        recipe_parent_dir.to_string(),
    );
    let mut missing_params: Vec<String> = Vec::new();
    for param in recipe_parameters.unwrap_or_default() {
        if !param_map.contains_key(&param.key) {
            match (&param.default, &param.requirement) {
                (Some(default), _) => param_map.insert(param.key.clone(), default.clone()),
                (None, RecipeParameterRequirement::UserPrompt) if user_prompt_fn.is_some() => {
                    let input_value =
                        user_prompt_fn.as_ref().unwrap()(&param.key, &param.description)?;
                    param_map.insert(param.key.clone(), input_value)
                }
                _ => {
                    missing_params.push(param.key.clone());
                    None
                }
            };
        }
    }
    Ok((param_map, missing_params))
}

fn resolve_sub_recipe_path(
    sub_recipe_path: &str,
    parent_recipe_dir: &Path,
) -> Result<String, RecipeError> {
    let path = if Path::new(sub_recipe_path).is_absolute() {
        sub_recipe_path.to_string()
    } else {
        parent_recipe_dir
            .join(sub_recipe_path)
            .to_str()
            .ok_or_else(|| RecipeError::RecipeParsing {
                source: anyhow::anyhow!("Invalid sub-recipe path: {}", sub_recipe_path),
            })?
            .to_string()
    };

    Ok(path)
}

#[cfg(test)]
mod tests;
