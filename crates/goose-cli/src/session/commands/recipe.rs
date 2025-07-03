use anyhow::{Context, Result};
use std::path::PathBuf;

use crate::session::{output, Session};

impl Session {
    pub async fn handle_recipe_command(&mut self, filepath_opt: Option<String>) -> Result<()> {
        println!("{}", console::style("Generating Recipe").green());

        output::show_thinking();
        let recipe = self.agent.create_recipe(self.messages.clone()).await;
        output::hide_thinking();

        match recipe {
            Ok(recipe) => {
                // Use provided filepath or default
                let filepath_str = filepath_opt.as_deref().unwrap_or("recipe.yaml");
                match self.save_recipe(&recipe, filepath_str) {
                    Ok(path) => println!(
                        "{}",
                        console::style(format!("Saved recipe to {}", path.display())).green()
                    ),
                    Err(e) => {
                        println!("{}", console::style(e).red());
                    }
                }
            }
            Err(e) => {
                println!(
                    "{}: {:?}",
                    console::style("Failed to generate recipe").red(),
                    e
                );
            }
        }

        Ok(())
    }

    pub fn save_recipe(
        &self,
        recipe: &goose::recipe::Recipe,
        filepath_str: &str,
    ) -> anyhow::Result<PathBuf> {
        let path_buf = PathBuf::from(filepath_str);
        let mut path = path_buf.clone();

        // Update the final path if it's relative
        if path_buf.is_relative() {
            // If the path is relative, resolve it relative to the current working directory
            let cwd = std::env::current_dir().context("Failed to get current directory")?;
            path = cwd.join(&path_buf);
        }

        // Check if parent directory exists
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                return Err(anyhow::anyhow!(
                    "Directory '{}' does not exist",
                    parent.display()
                ));
            }
        }

        // Try creating the file
        let file = std::fs::File::create(path.as_path())
            .context(format!("Failed to create file '{}'", path.display()))?;

        // Write YAML
        serde_yaml::to_writer(file, recipe).context("Failed to save recipe")?;

        Ok(path)
    }
}
