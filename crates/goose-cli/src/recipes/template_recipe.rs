use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use anyhow::Result;
use goose::recipe::Recipe;
use minijinja::{Environment, UndefinedBehavior};

use crate::recipes::recipe::BUILT_IN_RECIPE_DIR_PARAM;

const CURRENT_TEMPLATE_NAME: &str = "current_template";

pub fn render_recipe_content_with_params(
    content: &str,
    params: &HashMap<String, String>,
) -> Result<String> {
    let env = add_template_in_env(
        content,
        params.get(BUILT_IN_RECIPE_DIR_PARAM).unwrap().clone(),
        UndefinedBehavior::Strict,
    )?;
    let template = env.get_template(CURRENT_TEMPLATE_NAME).unwrap();
    let rendered_content = template
        .render(params)
        .map_err(|e| anyhow::anyhow!("Failed to render the recipe {}", e))?;
    Ok(rendered_content)
}

pub fn render_recipe_silent_when_variables_are_provided(
    content: &str,
    params: &HashMap<String, String>,
) -> Result<String> {
    let mut env = minijinja::Environment::new();
    env.set_undefined_behavior(UndefinedBehavior::Lenient);
    let template = env.template_from_str(content)?;
    let rendered_content = template.render(params)?;
    Ok(rendered_content)
}

fn add_template_in_env(
    content: &str,
    recipe_dir: String,
    undefined_behavior: UndefinedBehavior,
) -> Result<Environment> {
    let mut env = minijinja::Environment::new();
    env.set_undefined_behavior(undefined_behavior);
    env.set_loader(move |name| {
        let path = Path::new(recipe_dir.as_str()).join(name);
        match std::fs::read_to_string(&path) {
            Ok(content) => Ok(Some(content)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(minijinja::Error::new(
                minijinja::ErrorKind::InvalidOperation,
                "could not read template",
            )
            .with_source(e)),
        }
    });

    env.add_template(CURRENT_TEMPLATE_NAME, content)?;
    Ok(env)
}

fn get_env_with_template_variables(
    content: &str,
    recipe_dir: String,
    undefined_behavior: UndefinedBehavior,
) -> Result<(Environment, HashSet<String>)> {
    let env = add_template_in_env(content, recipe_dir, undefined_behavior)?;
    let template = env.get_template(CURRENT_TEMPLATE_NAME).unwrap();
    let state = template.eval_to_state(())?;
    let mut template_variables = HashSet::new();
    for (_, template) in state.env().templates() {
        template_variables.extend(template.undeclared_variables(true));
    }
    Ok((env, template_variables))
}

pub fn parse_recipe_content(
    content: &str,
    recipe_dir: String,
) -> Result<(Recipe, HashSet<String>)> {
    let (env, template_variables) =
        get_env_with_template_variables(content, recipe_dir, UndefinedBehavior::Lenient)?;
    let template = env.get_template(CURRENT_TEMPLATE_NAME).unwrap();
    let rendered_content = template
        .render(())
        .map_err(|e| anyhow::anyhow!("Failed to parse the recipe {}", e))?;
    let recipe = Recipe::from_content(&rendered_content)?;
    // return recipe (without loading any variables) and the variable names that are in the recipe
    Ok((recipe, template_variables))
}

// render the recipe for validation, deeplink and explain, etc.
pub fn render_recipe_for_preview(
    content: &str,
    recipe_dir: String,
    params: &HashMap<String, String>,
) -> Result<Recipe> {
    let (env, template_variables) =
        get_env_with_template_variables(content, recipe_dir, UndefinedBehavior::Lenient)?;
    let template = env.get_template(CURRENT_TEMPLATE_NAME).unwrap();
    // if the variables are not provided, the template will be rendered with the variables, otherwise it will keep the variables as is
    let mut ctx = preserve_vars(&template_variables).clone();
    ctx.extend(params.clone());
    let rendered_content = template
        .render(ctx)
        .map_err(|e| anyhow::anyhow!("Failed to parse the recipe {}", e))?;
    Recipe::from_content(&rendered_content)
}

fn preserve_vars(variables: &HashSet<String>) -> HashMap<String, String> {
    let mut context = HashMap::<String, String>::new();
    for template_var in variables {
        context.insert(template_var.clone(), format!("{{{{ {} }}}}", template_var));
    }
    context
}

#[cfg(test)]
mod tests {
    mod render_content_with_params_tests {
        use std::collections::HashMap;

        use crate::recipes::template_recipe::render_recipe_content_with_params;

        #[test]
        fn test_render_content_with_params() {
            // Test basic parameter substitution
            let content = "Hello {{ name }}!";
            let params = HashMap::from([
                ("recipe_dir".to_string(), "some_dir".to_string()),
                ("name".to_string(), "World".to_string()),
            ]);
            let result = render_recipe_content_with_params(content, &params).unwrap();
            assert_eq!(result, "Hello World!");

            // Test empty parameter substitution
            let content = "Hello {{ empty }}!";
            let params = HashMap::from([
                ("recipe_dir".to_string(), "some_dir".to_string()),
                ("empty".to_string(), "".to_string()),
            ]);
            let result = render_recipe_content_with_params(content, &params).unwrap();
            assert_eq!(result, "Hello !");

            // Test multiple parameters
            let content = "{{ greeting }} {{ name }}!";
            let params = HashMap::from([
                ("recipe_dir".to_string(), "some_dir".to_string()),
                ("greeting".to_string(), "Hi".to_string()),
                ("name".to_string(), "Alice".to_string()),
            ]);
            let result = render_recipe_content_with_params(content, &params).unwrap();
            assert_eq!(result, "Hi Alice!");

            // Test missing parameter results in error
            let content = "Hello {{ missing }}!";
            let params = HashMap::from([("recipe_dir".to_string(), "some_dir".to_string())]);
            let err = render_recipe_content_with_params(content, &params).unwrap_err();
            let error_msg = err.to_string();
            assert!(error_msg.contains("Failed to render the recipe"));

            // Test invalid template syntax results in error
            let content = "Hello {{ unclosed";
            let params = HashMap::from([("recipe_dir".to_string(), "some_dir".to_string())]);
            let err = render_recipe_content_with_params(content, &params).unwrap_err();
            assert!(err.to_string().contains("unexpected end of input"));
        }
    }
}
