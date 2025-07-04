use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Represents a single behavior/feature in the application plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanBehavior {
    /// Unique identifier for this behavior
    pub id: String,
    /// Human-readable name for this behavior
    pub name: String,
    /// Detailed instructions for implementing this behavior
    pub behavior: String,
}

/// Represents project metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    /// Title of the project
    pub title: String,
    /// Description of the project
    pub description: String,
    /// Primary programming language (defaults to "haskell")
    #[serde(default = "default_language")]
    pub language: String,
}

fn default_language() -> String {
    "haskell".to_string()
}

/// Represents the complete application plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    /// Project metadata
    pub project: ProjectInfo,
    /// List of behaviors/features that define the application
    pub behaviors: Vec<PlanBehavior>,
}

/// Manages plan.yaml detection and loading
pub struct PlanManager;

impl PlanManager {
    /// Check if a plan.yaml file exists in the given directory
    pub fn plan_exists_in_dir<P: AsRef<Path>>(dir: P) -> bool {
        let plan_path = dir.as_ref().join("plan.yaml");
        plan_path.exists()
    }

    /// Check if a plan.yaml file exists in the current working directory
    pub fn plan_exists_in_current_dir() -> bool {
        if let Ok(current_dir) = std::env::current_dir() {
            Self::plan_exists_in_dir(current_dir)
        } else {
            false
        }
    }

    /// Load a plan.yaml file from the given directory
    pub fn load_plan_from_dir<P: AsRef<Path>>(dir: P) -> Result<Plan> {
        let plan_path = dir.as_ref().join("plan.yaml");
        Self::load_plan_from_path(plan_path)
    }

    /// Load a plan.yaml file from the current working directory
    pub fn load_plan_from_current_dir() -> Result<Plan> {
        let current_dir = std::env::current_dir()
            .context("Failed to get current working directory")?;
        Self::load_plan_from_dir(current_dir)
    }

    /// Load a plan.yaml file from the specified path
    pub fn load_plan_from_path<P: AsRef<Path>>(path: P) -> Result<Plan> {
        let plan_path = path.as_ref();
        
        if !plan_path.exists() {
            return Err(anyhow::anyhow!(
                "Plan file does not exist: {}",
                plan_path.display()
            ));
        }

        let content = std::fs::read_to_string(plan_path)
            .with_context(|| format!("Failed to read plan file: {}", plan_path.display()))?;

        let plan: Plan = serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse plan file: {}", plan_path.display()))?;

        // Validate the plan
        Self::validate_plan(&plan)?;

        Ok(plan)
    }

    /// Validate that a plan has the required structure
    fn validate_plan(plan: &Plan) -> Result<()> {
        if plan.behaviors.is_empty() {
            return Err(anyhow::anyhow!("Plan must contain at least one behavior"));
        }

        // Check for duplicate IDs
        let mut ids = std::collections::HashSet::new();
        for behavior in &plan.behaviors {
            if behavior.id.is_empty() {
                return Err(anyhow::anyhow!("Behavior ID cannot be empty"));
            }
            if behavior.name.is_empty() {
                return Err(anyhow::anyhow!("Behavior name cannot be empty"));
            }
            if behavior.behavior.is_empty() {
                return Err(anyhow::anyhow!("Behavior instructions cannot be empty"));
            }
            
            if !ids.insert(&behavior.id) {
                return Err(anyhow::anyhow!(
                    "Duplicate behavior ID found: {}",
                    behavior.id
                ));
            }
        }

        Ok(())
    }

    /// Generate a system prompt from the plan behaviors and project info
    pub fn generate_system_prompt(plan: &Plan) -> String {
        let mut prompt = String::new();
        
        // Add project information
        prompt.push_str(&format!(
            "You are working on a project called \"{}\" ({}). Project description: {}\n\n",
            plan.project.title,
            plan.project.language,
            plan.project.description
        ));
        
        prompt.push_str("This application has the following defined behaviors:\n\n");
        
        for (index, behavior) in plan.behaviors.iter().enumerate() {
            prompt.push_str(&format!(
                "{}. **{}** (ID: {})\n   {}\n\n",
                index + 1,
                behavior.name,
                behavior.id,
                behavior.behavior
            ));
        }
        
        prompt.push_str("When implementing features or making changes, refer to these behaviors to ensure consistency with the application's intended functionality. ");
        prompt.push_str("Use the builder extension tools to implement these behaviors effectively.");
        
        prompt
    }

    /// Create a sample plan.yaml file for demonstration
    pub fn create_sample_plan<P: AsRef<Path>>(path: P) -> Result<()> {
        let sample_plan = Plan {
            project: ProjectInfo {
                title: "Sample Application".to_string(),
                description: "A sample web application demonstrating modern development practices with user authentication, data persistence, and a responsive frontend.".to_string(),
                language: "haskell".to_string(),
            },
            behaviors: vec![
                PlanBehavior {
                    id: "user_authentication".to_string(),
                    name: "User Authentication".to_string(),
                    behavior: "Implement secure user login and registration with email verification. Support OAuth providers like Google and GitHub. Include password reset functionality and session management.".to_string(),
                },
                PlanBehavior {
                    id: "data_persistence".to_string(),
                    name: "Data Persistence".to_string(),
                    behavior: "Set up database schema and models for storing user data, application state, and business logic. Use appropriate indexing and ensure data integrity with proper validation.".to_string(),
                },
                PlanBehavior {
                    id: "api_endpoints".to_string(),
                    name: "REST API Endpoints".to_string(),
                    behavior: "Create RESTful API endpoints for all core functionality. Include proper error handling, input validation, rate limiting, and comprehensive API documentation.".to_string(),
                },
                PlanBehavior {
                    id: "frontend_ui".to_string(),
                    name: "Frontend User Interface".to_string(),
                    behavior: "Build responsive web interface with modern UI components. Ensure accessibility compliance and cross-browser compatibility. Implement real-time updates where appropriate.".to_string(),
                },
            ],
        };

        let yaml_content = serde_yaml::to_string(&sample_plan)
            .context("Failed to serialize sample plan to YAML")?;

        std::fs::write(path.as_ref(), yaml_content)
            .with_context(|| format!("Failed to write sample plan to {}", path.as_ref().display()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_plan_validation() {
        // Test valid plan
        let valid_plan = Plan {
            project: ProjectInfo {
                title: "Test Project".to_string(),
                description: "A test project".to_string(),
                language: "haskell".to_string(),
            },
            behaviors: vec![
                PlanBehavior {
                    id: "test1".to_string(),
                    name: "Test Behavior 1".to_string(),
                    behavior: "This is a test behavior".to_string(),
                },
            ],
        };
        assert!(PlanManager::validate_plan(&valid_plan).is_ok());

        // Test empty behaviors
        let empty_plan = Plan {
            project: ProjectInfo {
                title: "Test Project".to_string(),
                description: "A test project".to_string(),
                language: "haskell".to_string(),
            },
            behaviors: vec![],
        };
        assert!(PlanManager::validate_plan(&empty_plan).is_err());

        // Test duplicate IDs
        let duplicate_plan = Plan {
            project: ProjectInfo {
                title: "Test Project".to_string(),
                description: "A test project".to_string(),
                language: "haskell".to_string(),
            },
            behaviors: vec![
                PlanBehavior {
                    id: "test1".to_string(),
                    name: "Test Behavior 1".to_string(),
                    behavior: "This is a test behavior".to_string(),
                },
                PlanBehavior {
                    id: "test1".to_string(),
                    name: "Test Behavior 2".to_string(),
                    behavior: "This is another test behavior".to_string(),
                },
            ],
        };
        assert!(PlanManager::validate_plan(&duplicate_plan).is_err());

        // Test empty fields
        let empty_id_plan = Plan {
            project: ProjectInfo {
                title: "Test Project".to_string(),
                description: "A test project".to_string(),
                language: "haskell".to_string(),
            },
            behaviors: vec![
                PlanBehavior {
                    id: "".to_string(),
                    name: "Test Behavior".to_string(),
                    behavior: "This is a test behavior".to_string(),
                },
            ],
        };
        assert!(PlanManager::validate_plan(&empty_id_plan).is_err());
    }

    #[test]
    fn test_plan_loading() {
        let temp_dir = TempDir::new().unwrap();
        let plan_path = temp_dir.path().join("plan.yaml");

        // Create a sample plan
        PlanManager::create_sample_plan(&plan_path).unwrap();

        // Test that plan exists
        assert!(PlanManager::plan_exists_in_dir(temp_dir.path()));

        // Test loading the plan
        let loaded_plan = PlanManager::load_plan_from_dir(temp_dir.path()).unwrap();
        assert_eq!(loaded_plan.behaviors.len(), 4);
        assert_eq!(loaded_plan.behaviors[0].id, "user_authentication");
    }

    #[test]
    fn test_system_prompt_generation() {
        let plan = Plan {
            project: ProjectInfo {
                title: "Test Project".to_string(),
                description: "A test application for demonstration".to_string(),
                language: "rust".to_string(),
            },
            behaviors: vec![
                PlanBehavior {
                    id: "test1".to_string(),
                    name: "Test Feature".to_string(),
                    behavior: "Implement a test feature with proper error handling".to_string(),
                },
            ],
        };

        let prompt = PlanManager::generate_system_prompt(&plan);
        assert!(prompt.contains("Test Project"));
        assert!(prompt.contains("rust"));
        assert!(prompt.contains("A test application for demonstration"));
        assert!(prompt.contains("Test Feature"));
        assert!(prompt.contains("test1"));
        assert!(prompt.contains("Implement a test feature"));
        assert!(prompt.contains("builder extension"));
    }

    #[test]
    fn test_plan_not_exists() {
        let temp_dir = TempDir::new().unwrap();
        assert!(!PlanManager::plan_exists_in_dir(temp_dir.path()));
        
        let result = PlanManager::load_plan_from_dir(temp_dir.path());
        assert!(result.is_err());
    }
}