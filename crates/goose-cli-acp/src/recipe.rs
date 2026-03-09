use std::path::Path;

use anyhow::{bail, Result};
use goose::recipe::Recipe;

pub fn load_recipe(path: &Path) -> Result<Recipe> {
    let recipe = Recipe::from_file_path(path)?;

    if recipe.check_for_security_warnings() {
        bail!(
            "Recipe '{}' contains potentially harmful unicode tag content",
            path.display()
        );
    }

    if recipe.instructions.is_none() && recipe.prompt.is_none() {
        bail!(
            "Recipe '{}' must have at least one of 'instructions' or 'prompt'",
            path.display()
        );
    }

    Ok(recipe)
}
