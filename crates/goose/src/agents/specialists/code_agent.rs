//! CodeAgent - Specialist agent for code generation and architecture

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{utils, SpecialistAgent, SpecialistConfig, SpecialistContext};
use crate::agents::orchestrator::{AgentRole, TaskResult};

/// Specialist agent focused on code generation and software architecture
pub struct CodeAgent {
    config: SpecialistConfig,
    capabilities: CodeCapabilities,
}

/// Capabilities of the code agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeCapabilities {
    /// Supported programming languages
    pub languages: Vec<String>,
    /// Supported frameworks
    pub frameworks: Vec<String>,
    /// Code patterns and architectures
    pub patterns: Vec<String>,
    /// Maximum lines of code to generate in one task
    pub max_lines_per_task: usize,
}

impl Default for CodeCapabilities {
    fn default() -> Self {
        Self {
            languages: vec![
                "rust".to_string(),
                "python".to_string(),
                "javascript".to_string(),
                "typescript".to_string(),
                "go".to_string(),
                "java".to_string(),
                "cpp".to_string(),
                "c".to_string(),
            ],
            frameworks: vec![
                "react".to_string(),
                "vue".to_string(),
                "angular".to_string(),
                "nextjs".to_string(),
                "django".to_string(),
                "flask".to_string(),
                "fastapi".to_string(),
                "express".to_string(),
                "spring".to_string(),
                "actix".to_string(),
                "tokio".to_string(),
            ],
            patterns: vec![
                "mvc".to_string(),
                "mvp".to_string(),
                "mvvm".to_string(),
                "repository".to_string(),
                "factory".to_string(),
                "observer".to_string(),
                "singleton".to_string(),
                "dependency_injection".to_string(),
                "clean_architecture".to_string(),
                "hexagonal".to_string(),
            ],
            max_lines_per_task: 1000,
        }
    }
}

impl CodeAgent {
    /// Create a new CodeAgent with configuration
    pub fn new(config: SpecialistConfig) -> Self {
        Self {
            config,
            capabilities: CodeCapabilities::default(),
        }
    }

    /// Create a CodeAgent with custom capabilities
    pub fn with_capabilities(config: SpecialistConfig, capabilities: CodeCapabilities) -> Self {
        Self {
            config,
            capabilities,
        }
    }

    /// Analyze code requirements from context
    fn analyze_requirements(&self, context: &SpecialistContext) -> CodeRequirements {
        let language = context
            .language
            .clone()
            .or_else(|| utils::detect_language(&context.target_files));
        let framework = context
            .framework
            .clone()
            .or_else(|| utils::detect_framework(&context.target_files, language.as_deref()));

        let complexity = utils::estimate_complexity(context);
        let estimated_lines = std::cmp::min(
            (complexity * 10) as usize,
            self.capabilities.max_lines_per_task,
        );

        CodeRequirements {
            language,
            framework,
            complexity,
            estimated_lines,
            requires_tests: context
                .metadata
                .get("require_tests")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
            requires_docs: context
                .metadata
                .get("require_docs")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            architecture_pattern: context
                .metadata
                .get("architecture")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        }
    }

    /// Generate code based on requirements
    async fn generate_code(
        &self,
        context: &SpecialistContext,
        requirements: &CodeRequirements,
    ) -> Result<Vec<GeneratedFile>> {
        let mut files = Vec::new();

        // For now, simulate code generation
        // In a real implementation, this would use the LLM to generate actual code
        match requirements.language.as_deref() {
            Some("rust") => {
                files.extend(self.generate_rust_code(context, requirements).await?);
            }
            Some("python") => {
                files.extend(self.generate_python_code(context, requirements).await?);
            }
            Some("javascript") | Some("typescript") => {
                files.extend(self.generate_js_code(context, requirements).await?);
            }
            _ => {
                files.push(GeneratedFile {
                    path: format!("{}/main.txt", context.working_dir),
                    content: format!(
                        "// Generated code for: {}\n// Language: {:?}\n// Framework: {:?}",
                        context.task, requirements.language, requirements.framework
                    ),
                    language: requirements.language.clone(),
                });
            }
        }

        Ok(files)
    }

    /// Generate Rust-specific code
    async fn generate_rust_code(
        &self,
        context: &SpecialistContext,
        _requirements: &CodeRequirements,
    ) -> Result<Vec<GeneratedFile>> {
        let mut files = Vec::new();

        // Main implementation file
        let main_content = format!(
            r#"//! {}

use anyhow::Result;
use serde::{{Serialize, Deserialize}};

/// Main structure for {}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MainComponent {{
    // TODO: Add fields based on requirements
}}

impl MainComponent {{
    /// Create a new instance
    pub fn new() -> Self {{
        Self {{
            // TODO: Initialize fields
        }}
    }}

    /// Main functionality
    pub async fn execute(&self) -> Result<()> {{
        // TODO: Implement main logic for: {}
        Ok(())
    }}
}}

#[cfg(test)]
mod tests {{
    use super::*;

    #[tokio::test]
    async fn test_main_component() {{
        let component = MainComponent::new();
        assert!(component.execute().await.is_ok());
    }}
}}
"#,
            context.task,
            context.task.replace(" ", "_"),
            context.task
        );

        files.push(GeneratedFile {
            path: format!("{}/src/main.rs", context.working_dir),
            content: main_content,
            language: Some("rust".to_string()),
        });

        // Cargo.toml if needed
        if !context
            .target_files
            .iter()
            .any(|f| f.ends_with("Cargo.toml"))
        {
            let cargo_content = r#"[package]
name = "generated_project"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = "1.0"
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1.0", features = ["full"] }

[dev-dependencies]
tokio-test = "0.4"
"#
            .to_string();

            files.push(GeneratedFile {
                path: format!("{}/Cargo.toml", context.working_dir),
                content: cargo_content,
                language: Some("toml".to_string()),
            });
        }

        Ok(files)
    }

    /// Generate Python-specific code
    async fn generate_python_code(
        &self,
        context: &SpecialistContext,
        _requirements: &CodeRequirements,
    ) -> Result<Vec<GeneratedFile>> {
        let mut files = Vec::new();

        let main_content = format!(
            r#"""{}"""

from typing import Optional, Dict, Any
import asyncio
import logging

logger = logging.getLogger(__name__)


class MainComponent:
    """Main component for {}"""
    
    def __init__(self):
        """Initialize the component"""
        # TODO: Add initialization based on requirements
        pass
    
    async def execute(self) -> Dict[str, Any]:
        """Execute main functionality"""
        # TODO: Implement main logic for: {}
        logger.info("Executing main component")
        return {{"status": "completed"}}


async def main():
    """Main entry point"""
    component = MainComponent()
    result = await component.execute()
    print(f"Result: {{result}}")


if __name__ == "__main__":
    asyncio.run(main())
"#,
            context.task, context.task, context.task
        );

        files.push(GeneratedFile {
            path: format!("{}/main.py", context.working_dir),
            content: main_content,
            language: Some("python".to_string()),
        });

        // Requirements file if needed
        if !context
            .target_files
            .iter()
            .any(|f| f.ends_with("requirements.txt"))
        {
            let requirements_content =
                "# Generated requirements\naiohttp>=3.8.0\npydantic>=1.10.0\n";
            files.push(GeneratedFile {
                path: format!("{}/requirements.txt", context.working_dir),
                content: requirements_content.to_string(),
                language: Some("text".to_string()),
            });
        }

        Ok(files)
    }

    /// Generate JavaScript/TypeScript code
    async fn generate_js_code(
        &self,
        context: &SpecialistContext,
        requirements: &CodeRequirements,
    ) -> Result<Vec<GeneratedFile>> {
        let mut files = Vec::new();

        let is_typescript = requirements.language.as_deref() == Some("typescript");
        let extension = if is_typescript { "ts" } else { "js" };

        let main_content = format!(
            r#"/**
 * {}
 */

{}

export class MainComponent {{
    constructor() {{
        // TODO: Initialize based on requirements
    }}

    {}execute(){}{{
        // TODO: Implement main logic for: {}
        console.log('Executing main component');
        return {{ status: 'completed' }};
    }}
}}

// Usage example
{}function main(){}{{
    const component = new MainComponent();
    {}result = {}component.execute();
    console.log('Result:', result);
}}

if (typeof require !== 'undefined' && require.main === module) {{
    main();
}}
"#,
            context.task,
            if is_typescript {
                "interface ExecutionResult {\n    status: string;\n    [key: string]: any;\n}"
            } else {
                ""
            },
            if is_typescript { "async " } else { "" },
            if is_typescript {
                ": Promise<ExecutionResult> "
            } else {
                " "
            },
            context.task,
            if is_typescript { "async " } else { "" },
            if is_typescript {
                ": Promise<void> "
            } else {
                " "
            },
            "const ",
            if is_typescript { "await " } else { "" }
        );

        files.push(GeneratedFile {
            path: format!("{}/main.{}", context.working_dir, extension),
            content: main_content,
            language: requirements.language.clone(),
        });

        // Package.json if needed
        if !context
            .target_files
            .iter()
            .any(|f| f.ends_with("package.json"))
        {
            let package_content = format!(
                r#"{{
  "name": "generated-project",
  "version": "1.0.0",
  "description": "{}",
  "main": "main.{}",
  "scripts": {{
    "start": "node main.{}",
    "test": "echo \"Error: no test specified\" && exit 1"
  }},
  "keywords": [],
  "author": "",
  "license": "MIT"{}
}}
"#,
                context.task,
                extension,
                extension,
                if is_typescript {
                    ",\n  \"devDependencies\": {\n    \"typescript\": \"^5.0.0\",\n    \"@types/node\": \"^20.0.0\"\n  }"
                } else {
                    ""
                }
            );

            files.push(GeneratedFile {
                path: format!("{}/package.json", context.working_dir),
                content: package_content,
                language: Some("json".to_string()),
            });
        }

        Ok(files)
    }
}

#[async_trait]
impl SpecialistAgent for CodeAgent {
    fn role(&self) -> AgentRole {
        AgentRole::Code
    }

    fn name(&self) -> &str {
        "CodeAgent"
    }

    async fn can_handle(&self, context: &SpecialistContext) -> bool {
        // Check if we can handle the programming language
        let language = context
            .language
            .clone()
            .or_else(|| utils::detect_language(&context.target_files));

        if let Some(lang) = language {
            self.capabilities.languages.contains(&lang)
        } else {
            // If no language detected, assume we can handle it
            true
        }
    }

    async fn execute(&self, context: SpecialistContext) -> Result<TaskResult> {
        let requirements = self.analyze_requirements(&context);

        tracing::info!(
            "CodeAgent executing: {} (Language: {:?})",
            context.task,
            requirements.language
        );

        let generated_files = self.generate_code(&context, &requirements).await?;

        let mut files_modified = Vec::new();
        let mut artifacts = Vec::new();

        // Simulate writing files
        for file in &generated_files {
            files_modified.push(file.path.clone());
            artifacts.push(format!(
                "Generated {} ({} lines)",
                file.path,
                file.content.lines().count()
            ));
        }

        let mut metrics = HashMap::new();
        metrics.insert(
            "files_generated".to_string(),
            serde_json::Value::Number(generated_files.len().into()),
        );
        metrics.insert(
            "estimated_lines".to_string(),
            serde_json::Value::Number(requirements.estimated_lines.into()),
        );
        metrics.insert(
            "complexity".to_string(),
            serde_json::Value::Number(requirements.complexity.into()),
        );

        let result = TaskResult {
            success: true,
            output: format!(
                "Generated {} files for: {}",
                generated_files.len(),
                context.task
            ),
            files_modified,
            artifacts,
            metrics,
        };

        tracing::info!(
            "CodeAgent completed successfully: {} files generated",
            generated_files.len()
        );
        Ok(result)
    }

    fn config(&self) -> &SpecialistConfig {
        &self.config
    }

    async fn estimate_duration(&self, context: &SpecialistContext) -> std::time::Duration {
        let requirements = self.analyze_requirements(context);
        let base_time = std::time::Duration::from_secs(600); // 10 minutes base
        let complexity_factor = requirements.complexity as u64 * 30; // 30 seconds per complexity point
        let lines_factor = (requirements.estimated_lines / 50) as u64 * 60; // 1 minute per 50 lines

        base_time + std::time::Duration::from_secs(complexity_factor + lines_factor)
    }

    async fn validate_result(&self, result: &TaskResult) -> Result<bool> {
        // Validate that files were actually generated
        if result.files_modified.is_empty() {
            return Ok(false);
        }

        // Check that the output indicates success
        if !result.output.contains("Generated") {
            return Ok(false);
        }

        Ok(result.success)
    }
}

/// Code generation requirements analysis
#[derive(Debug, Clone)]
struct CodeRequirements {
    language: Option<String>,
    framework: Option<String>,
    complexity: u32,
    estimated_lines: usize,
    #[allow(dead_code)]
    requires_tests: bool,
    #[allow(dead_code)]
    requires_docs: bool,
    #[allow(dead_code)]
    architecture_pattern: Option<String>,
}

/// Generated code file
#[derive(Debug, Clone)]
struct GeneratedFile {
    path: String,
    content: String,
    #[allow(dead_code)]
    language: Option<String>,
}
