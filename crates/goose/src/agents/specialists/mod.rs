//! Specialist agents for different development domains
//!
//! This module contains specialist agent implementations that focus on specific
//! aspects of software development: code generation, testing, deployment,
//! documentation, and security.

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::agents::orchestrator::{AgentRole, TaskResult};

pub mod code_agent;
pub mod deploy_agent;
pub mod docs_agent;
pub mod security_agent;
pub mod test_agent;

pub use code_agent::CodeAgent;
pub use deploy_agent::DeployAgent;
pub use docs_agent::DocsAgent;
pub use security_agent::SecurityAgent;
pub use test_agent::TestAgent;

/// Context information for specialist agent execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecialistContext {
    /// The task description
    pub task: String,
    /// Working directory for the task
    pub working_dir: String,
    /// Files currently being worked on
    pub target_files: Vec<String>,
    /// Previous task outputs that this task depends on
    pub dependencies: HashMap<String, TaskResult>,
    /// Additional metadata for the task
    pub metadata: HashMap<String, serde_json::Value>,
    /// Language/framework context
    pub language: Option<String>,
    /// Framework context (e.g., "react", "django", "express")
    pub framework: Option<String>,
    /// Environment (development, staging, production)
    pub environment: Option<String>,
}

impl SpecialistContext {
    pub fn new(task: String, working_dir: String) -> Self {
        Self {
            task,
            working_dir,
            target_files: Vec::new(),
            dependencies: HashMap::new(),
            metadata: HashMap::new(),
            language: None,
            framework: None,
            environment: None,
        }
    }

    pub fn with_files(mut self, files: Vec<String>) -> Self {
        self.target_files = files;
        self
    }

    pub fn with_language(mut self, language: String) -> Self {
        self.language = Some(language);
        self
    }

    pub fn with_framework(mut self, framework: String) -> Self {
        self.framework = Some(framework);
        self
    }

    pub fn with_dependency(mut self, name: String, result: TaskResult) -> Self {
        self.dependencies.insert(name, result);
        self
    }

    pub fn with_metadata(mut self, key: String, value: serde_json::Value) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Configuration for specialist agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecialistConfig {
    /// Maximum execution time for tasks
    pub timeout: std::time::Duration,
    /// Whether to enable verbose logging
    pub verbose: bool,
    /// Custom tool configurations
    pub tools: HashMap<String, serde_json::Value>,
    /// Environment variables for execution
    pub env_vars: HashMap<String, String>,
}

impl Default for SpecialistConfig {
    fn default() -> Self {
        Self {
            timeout: std::time::Duration::from_secs(1800), // 30 minutes
            verbose: false,
            tools: HashMap::new(),
            env_vars: HashMap::new(),
        }
    }
}

/// Trait for specialist agents
#[async_trait]
pub trait SpecialistAgent: Send + Sync {
    /// Get the role/specialty of this agent
    fn role(&self) -> AgentRole;

    /// Get the agent's name
    fn name(&self) -> &str;

    /// Check if this agent can handle the given task
    async fn can_handle(&self, context: &SpecialistContext) -> bool;

    /// Execute a task within this agent's specialty
    async fn execute(&self, context: SpecialistContext) -> Result<TaskResult>;

    /// Get agent-specific configuration
    fn config(&self) -> &SpecialistConfig;

    /// Estimate how long a task will take
    async fn estimate_duration(&self, context: &SpecialistContext) -> std::time::Duration {
        // Default estimation based on task complexity
        let base_time = std::time::Duration::from_secs(300); // 5 minutes
        let file_factor = context.target_files.len() as u64 * 60; // 1 min per file
        let complexity_factor = context.task.len() as u64; // 1 sec per character

        base_time + std::time::Duration::from_secs(file_factor + complexity_factor)
    }

    /// Validate task results before completion
    async fn validate_result(&self, result: &TaskResult) -> Result<bool> {
        // Basic validation - check if the task succeeded
        Ok(result.success)
    }

    /// Clean up after task execution
    async fn cleanup(&self) -> Result<()> {
        // Default: no cleanup needed
        Ok(())
    }
}

/// Factory for creating specialist agents
pub struct SpecialistFactory;

impl SpecialistFactory {
    /// Create a specialist agent for the given role
    pub fn create(
        role: AgentRole,
        config: Option<SpecialistConfig>,
    ) -> Result<Box<dyn SpecialistAgent>> {
        let config = config.unwrap_or_default();

        match role {
            AgentRole::Code => Ok(Box::new(CodeAgent::new(config))),
            AgentRole::Test => Ok(Box::new(TestAgent::new(config))),
            AgentRole::Deploy => Ok(Box::new(DeployAgent::new(config))),
            AgentRole::Docs => Ok(Box::new(DocsAgent::new(config))),
            AgentRole::Security => Ok(Box::new(SecurityAgent::new(config))),
            AgentRole::Coordinator => Err(anyhow::anyhow!(
                "Coordinator role should use AgentOrchestrator"
            )),
        }
    }

    /// Create all default specialist agents
    pub fn create_all() -> Result<HashMap<AgentRole, Box<dyn SpecialistAgent>>> {
        let mut agents = HashMap::new();

        let roles = [
            AgentRole::Code,
            AgentRole::Test,
            AgentRole::Deploy,
            AgentRole::Docs,
            AgentRole::Security,
        ];

        for role in roles {
            let agent = Self::create(role, None)?;
            agents.insert(role, agent);
        }

        Ok(agents)
    }
}

/// Utility functions for specialist agents
pub mod utils {
    use super::*;
    use std::path::Path;

    /// Detect programming language from file extensions
    pub fn detect_language(files: &[String]) -> Option<String> {
        let mut language_counts = HashMap::new();

        for file in files {
            if let Some(ext) = Path::new(file).extension() {
                let lang = match ext.to_str()? {
                    "rs" => "rust",
                    "py" => "python",
                    "js" | "jsx" => "javascript",
                    "ts" | "tsx" => "typescript",
                    "go" => "go",
                    "java" => "java",
                    "cpp" | "cc" | "cxx" => "cpp",
                    "c" => "c",
                    "cs" => "csharp",
                    "rb" => "ruby",
                    "php" => "php",
                    "swift" => "swift",
                    "kt" => "kotlin",
                    "scala" => "scala",
                    "clj" => "clojure",
                    "hs" => "haskell",
                    "ml" => "ocaml",
                    "elm" => "elm",
                    "dart" => "dart",
                    "lua" => "lua",
                    "pl" => "perl",
                    "r" => "r",
                    "jl" => "julia",
                    "nim" => "nim",
                    "zig" => "zig",
                    _ => continue,
                };

                *language_counts.entry(lang.to_string()).or_insert(0) += 1;
            }
        }

        // Return the most common language
        language_counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(lang, _)| lang)
    }

    /// Detect framework from file patterns and content
    pub fn detect_framework(files: &[String], language: Option<&str>) -> Option<String> {
        // Check for common framework indicators
        for file in files {
            let file_name = Path::new(file).file_name()?.to_str()?;

            match file_name {
                "package.json" => return Some("node".to_string()),
                "Cargo.toml" => return Some("rust".to_string()),
                "requirements.txt" | "pyproject.toml" => return Some("python".to_string()),
                "go.mod" => return Some("go".to_string()),
                "pom.xml" | "build.gradle" => return Some("java".to_string()),
                "Gemfile" => return Some("ruby".to_string()),
                "composer.json" => return Some("php".to_string()),
                _ => {}
            }
        }

        // Framework-specific file patterns
        if files.iter().any(|f| f.contains("next.config")) {
            return Some("nextjs".to_string());
        }
        if files.iter().any(|f| f.contains("nuxt.config")) {
            return Some("nuxt".to_string());
        }
        if files.iter().any(|f| f.contains("vue.config")) {
            return Some("vue".to_string());
        }
        if files.iter().any(|f| f.contains("angular.json")) {
            return Some("angular".to_string());
        }
        if files.iter().any(|f| f.contains("manage.py")) {
            return Some("django".to_string());
        }
        if files
            .iter()
            .any(|f| f.contains("app.py") || f.contains("main.py"))
            && language == Some("python")
        {
            return Some("flask".to_string());
        }
        if files.iter().any(|f| f.contains("src/main.rs")) {
            return Some("rust".to_string());
        }

        None
    }

    /// Estimate task complexity based on various factors
    pub fn estimate_complexity(context: &SpecialistContext) -> u32 {
        let mut complexity = 0;

        // Base complexity from task description length
        complexity += (context.task.len() / 100) as u32;

        // File count factor
        complexity += context.target_files.len() as u32 * 2;

        // Dependency factor
        complexity += context.dependencies.len() as u32 * 3;

        // Language/framework complexity
        match context.language.as_deref() {
            Some("rust") | Some("cpp") | Some("c") => complexity += 10, // Systems languages
            Some("javascript") | Some("python") => complexity += 5,     // Dynamic languages
            Some("go") | Some("java") => complexity += 7,               // Compiled languages
            _ => complexity += 5,                                       // Default
        }

        // Cap complexity at reasonable levels
        std::cmp::min(complexity, 100)
    }
}
