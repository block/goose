use std::collections::HashMap;

use console::style;
use goose::recipe::Recipe;

pub fn print_recipe_explanation(recipe: &Recipe) {
    println!(
        "{} {}",
        style("🔍 Loading recipe:").bold().green(),
        style(&recipe.title).green()
    );
    println!("{}", style("📄 Description:").bold());
    println!("   {}", recipe.description);
    if let Some(params) = &recipe.parameters {
        if !params.is_empty() {
            println!("{}", style("⚙️  Recipe Parameters:").bold());
            for param in params {
                let default_display = match &param.default {
                    Some(val) => format!(" (default: {})", val),
                    None => String::new(),
                };

                println!(
                    "   - {} ({}, {}){}: {}",
                    style(&param.key).cyan(),
                    param.input_type,
                    param.requirement,
                    default_display,
                    param.description
                );
            }
        }
    }
}

pub fn print_required_parameters_for_template(
    params_for_template: HashMap<String, String>,
    missing_params: Vec<String>,
) {
    if !params_for_template.is_empty() {
        println!(
            "{}",
            style("📥 Parameters used to load this recipe:").bold()
        );
        for (key, value) in params_for_template {
            println!("   {}: {}", key, value);
        }
    }
    if !missing_params.is_empty() {
        println!(
            "{}",
            style("🔴 Missing parameters in the command line if you want to run the recipe:")
                .bold()
        );
        for param in missing_params.iter() {
            println!("   - {}", param);
        }
        println!(
            "📩 {}:",
            style("Please provide the following parameters in the command line if you want to run the recipe:").bold()
        );
        println!("  {}", missing_parameters_command_line(missing_params));
    }
}

pub fn missing_parameters_command_line(missing_params: Vec<String>) -> String {
    missing_params
        .iter()
        .map(|key| format!("--params {}=your_value", key))
        .collect::<Vec<_>>()
        .join(" ")
}
