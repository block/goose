use goose::recipe::{Recipe, SubRecipe};

pub fn create_sub_recipe_instructions(recipe: &Recipe) -> String {
    let mut instructions = String::new();
    if let Some(sub_recipes) = &recipe.sub_recipes {
        for sub_recipe in sub_recipes {
            let sub_recipe_command = create_sub_recipe_command(sub_recipe);
            instructions.push_str(&format!("if {} is required to run, then run command: {} \n", sub_recipe.name, sub_recipe_command));
        }
    }
    instructions
}

pub fn create_sub_recipe_command(sub_recipe: &SubRecipe) -> String {
    let mut command = String::new();
    command.push_str(&format!("goose run --recipe {} ", sub_recipe.file));
    if let Some(params) = &sub_recipe.params {
        for param in params {
            command.push_str(&format!(" --params {}={}", param.name, param.value));
        }
    }
    return command;
}