use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GooseApp {
    pub name: String,
    pub description: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub resizable: Option<bool>,
    pub prd: String,
    pub js_implementation: String,
}

impl GooseApp {
    // goose aps are stored in frontmatter format, with the delimiter being "---"
    // name: GooseApp
    // description: Optional description of the app
    // ---
    // JavaScript implementation of the app
    const FRONTMATTER_DELIMITER: &'static str = "\n---\n";

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let parts: Vec<&str> = content.splitn(2, Self::FRONTMATTER_DELIMITER).collect();

        if parts.len() != 2 {
            return Err(anyhow::anyhow!(
                "Invalid app file format - missing frontmatter delimiter"
            ));
        }

        let mut app: GooseApp = serde_yaml::from_str(parts[0])?;
        app.js_implementation = parts[1].to_string();

        Ok(app)
    }

    pub fn to_file_content(&self) -> Result<String> {
        let mut metadata = self.clone();
        metadata.js_implementation = String::new();
        let yaml_content = serde_yaml::to_string(&metadata)?;
        Ok(format!(
            "{}{}{}",
            yaml_content,
            Self::FRONTMATTER_DELIMITER,
            self.js_implementation
        ))
    }
}
