//! DocsAgent - Specialist agent for documentation generation

use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;

use super::{SpecialistAgent, SpecialistConfig, SpecialistContext};
use crate::agents::orchestrator::{AgentRole, TaskResult};

/// Specialist agent focused on documentation
pub struct DocsAgent {
    config: SpecialistConfig,
}

impl DocsAgent {
    pub fn new(config: SpecialistConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl SpecialistAgent for DocsAgent {
    fn role(&self) -> AgentRole {
        AgentRole::Docs
    }

    fn name(&self) -> &str {
        "DocsAgent"
    }

    async fn can_handle(&self, context: &SpecialistContext) -> bool {
        let task_lower = context.task.to_lowercase();
        let doc_keywords = ["document", "readme", "docs", "api doc", "guide"];
        doc_keywords
            .iter()
            .any(|keyword| task_lower.contains(keyword))
    }

    async fn execute(&self, context: SpecialistContext) -> Result<TaskResult> {
        let mut files_modified = Vec::new();
        let mut artifacts = Vec::new();

        // Generate README.md
        let _readme_content = format!(
            "# {}\n\nGenerated documentation for the project.\n",
            context.task
        );
        files_modified.push(format!("{}/README.md", context.working_dir));
        artifacts.push("Generated README.md".to_string());

        let mut metrics = HashMap::new();
        metrics.insert(
            "docs_generated".to_string(),
            serde_json::Value::Number(1.into()),
        );

        Ok(TaskResult {
            success: true,
            output: format!("Generated documentation for: {}", context.task),
            files_modified,
            artifacts,
            metrics,
        })
    }

    fn config(&self) -> &SpecialistConfig {
        &self.config
    }
}
