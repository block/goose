use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json;
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_server: Option<String>,
    #[serde(default)]
    pub html: String,
    #[serde(default)]
    pub prd: String,
}

impl GooseApp {
    const METADATA_SCRIPT_TYPE: &'static str = "application/ld+json";
    const PRD_SCRIPT_TYPE: &'static str = "application/x-goose-prd";
    const GOOSE_APP_TYPE: &'static str = "GooseApp";
    const GOOSE_SCHEMA_CONTEXT: &'static str = "https://goose.ai/schema";

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let html = fs::read_to_string(path)?;
        Self::from_html(&html)
    }

    pub fn from_html(html: &str) -> Result<Self> {
        let metadata_re = regex::Regex::new(&format!(
            r#"(?s)<script type="{}"[^>]*>\s*(.*?)\s*</script>"#,
            regex::escape(Self::METADATA_SCRIPT_TYPE)
        ))?;

        let prd_re = regex::Regex::new(&format!(
            r#"(?s)<script type="{}"[^>]*>\s*(.*?)\s*</script>"#,
            regex::escape(Self::PRD_SCRIPT_TYPE)
        ))?;

        let json_str = metadata_re
            .captures(html)
            .and_then(|cap| cap.get(1))
            .ok_or_else(|| anyhow::anyhow!("No GooseApp JSON-LD metadata found in HTML"))?
            .as_str();

        let mut app: GooseApp = serde_json::from_str(json_str)?;

        app.prd = prd_re
            .captures(html)
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str().trim().to_string())
            .unwrap_or_default();

        let clean_html = metadata_re.replace(html, "");
        app.html = prd_re.replace(&clean_html, "").to_string();

        Ok(app)
    }

    pub fn to_file_content(&self) -> Result<String> {
        let metadata_json = serde_json::to_string_pretty(&serde_json::json!({
            "@context": Self::GOOSE_SCHEMA_CONTEXT,
            "@type": Self::GOOSE_APP_TYPE,
            "name": self.name,
            "description": self.description,
            "width": self.width,
            "height": self.height,
            "resizable": self.resizable,
            "mcpServer": self.mcp_server,
        }))?;

        let metadata_script = format!(
            "<script type=\"{}\">\n{}\n</script>",
            Self::METADATA_SCRIPT_TYPE,
            metadata_json
        );

        let prd_script = if !self.prd.is_empty() {
            format!(
                "<script type=\"{}\">\n{}\n</script>",
                Self::PRD_SCRIPT_TYPE,
                self.prd
            )
        } else {
            String::new()
        };

        let scripts = format!("{}\n{}\n", metadata_script, prd_script);

        let result = if let Some(head_pos) = self.html.find("</head>") {
            let mut html = self.html.clone();
            html.insert_str(head_pos, &scripts);
            html
        } else if let Some(html_pos) = self.html.find("<html") {
            let after_html = self
                .html
                .get(html_pos..)
                .and_then(|s| s.find('>'))
                .map(|p| html_pos + p + 1);
            if let Some(pos) = after_html {
                let mut html = self.html.clone();
                html.insert_str(pos, &format!("\n<head>\n{}</head>", scripts));
                html
            } else {
                format!("<head>\n{}</head>\n{}", scripts, self.html)
            }
        } else {
            format!(
                "<html>\n<head>\n{}</head>\n<body>\n{}\n</body>\n</html>",
                scripts, self.html
            )
        };

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clock_html() {
        let html = include_str!("clock.html");
        let app = GooseApp::from_html(html).expect("Failed to parse clock.html");

        assert_eq!(app.name, "Clock");
        assert_eq!(
            app.description,
            Some("A simple clock with 12/24 hour toggle".to_string())
        );
        assert_eq!(app.width, Some(400));
        assert_eq!(app.height, Some(300));
        assert_eq!(app.resizable, Some(true));
        assert!(!app.prd.is_empty());
        assert!(app.html.contains("<body>"));
        assert!(!app.html.contains("application/ld+json"));
        assert!(!app.html.contains("application/x-goose-prd"));
    }
}
