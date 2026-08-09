use super::discover_skills;
use super::loaded_skill_context_with_args;
use crate::agents::extension::PlatformExtensionContext;
use crate::agents::mcp_client::{Error, McpClientTrait};
use crate::agents::ToolCallContext;
use async_trait::async_trait;
use goose_sdk_types::custom_requests::{SourceEntry, SourceType};
use rmcp::model::{
    CallToolResult, ContentBlock, Implementation, InitializeResult, JsonObject, ListToolsResult,
    ServerCapabilities, ServerNotification, Tool,
};
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

pub static EXTENSION_NAME: &str = "skills";

pub struct SkillsClient {
    info: InitializeResult,
    working_dir: PathBuf,
    exclude_builtin_skills: bool,
}

impl SkillsClient {
    pub fn new(context: PlatformExtensionContext) -> anyhow::Result<Self> {
        let working_dir = context
            .session
            .as_ref()
            .map(|s| s.working_dir.clone())
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        let info = InitializeResult::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(EXTENSION_NAME, "1.0.0").with_title("Skills"));

        Ok(Self {
            info,
            working_dir,
            exclude_builtin_skills: false,
        })
    }

    /// Controls whether Goose's bundled skills are exposed by this client.
    /// Bundled skills are enabled by default.
    pub fn with_builtin_skills(mut self, enabled: bool) -> Self {
        self.exclude_builtin_skills = !enabled;
        self
    }

    fn discover_skills(&self) -> Vec<SourceEntry> {
        discover_skills(Some(&self.working_dir))
            .into_iter()
            .filter(|skill| {
                !self.exclude_builtin_skills || skill.source_type != SourceType::BuiltinSkill
            })
            .collect()
    }

    fn handle_create_skill(&self, arguments: Option<JsonObject>) -> Result<CallToolResult, Error> {
        let args = arguments.as_ref();
        let name = args
            .and_then(|a| a.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        let description = args
            .and_then(|a| a.get("description"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        let content = args
            .and_then(|a| a.get("content"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let global = args
            .and_then(|a| a.get("global"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if name.is_empty() {
            return Ok(CallToolResult::error(vec![ContentBlock::text(
                "Missing required parameter: name",
            )]));
        }
        if description.is_empty() {
            return Ok(CallToolResult::error(vec![ContentBlock::text(
                "Missing required parameter: description",
            )]));
        }
        if content.trim().is_empty() {
            return Ok(CallToolResult::error(vec![ContentBlock::text(
                "Missing required parameter: content",
            )]));
        }

        let project_dir = if global {
            None
        } else {
            Some(self.working_dir.to_string_lossy().into_owned())
        };

        match crate::sources::create_source(
            SourceType::Skill,
            name,
            description,
            content,
            global,
            project_dir.as_deref(),
            std::collections::HashMap::new(),
        ) {
            Ok(source) => {
                let path = source.path.replace('\\', "/");
                Ok(CallToolResult::success(vec![ContentBlock::text(format!(
                    "Created skill '{}' at {}.\n\n\
                     Verify discovery with /skills or the Skills page. \
                     Prefer create_skill for future skills — never write to skills/ or workspace/skills/.",
                    source.name, path
                ))]))
            }
            Err(e) => {
                let message = match &e.data {
                    Some(serde_json::Value::String(s)) => s.clone(),
                    Some(other) => other.to_string(),
                    None => e.message.clone(),
                };
                Ok(CallToolResult::error(vec![ContentBlock::text(format!(
                    "Failed to create skill: {}",
                    message
                ))]))
            }
        }
    }

    fn handle_load_skill(&self, arguments: Option<JsonObject>) -> Result<CallToolResult, Error> {
        let skill_name = arguments
            .as_ref()
            .and_then(|args| args.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if skill_name.is_empty() {
            return Ok(CallToolResult::error(vec![ContentBlock::text(
                "Missing required parameter: name",
            )]));
        }
        let args = arguments
            .as_ref()
            .and_then(|args| args.get("args"))
            .and_then(|v| v.as_str());

        let skills = self.discover_skills();

        if let Some(skill) = skills.iter().find(|s| s.name == skill_name) {
            return match loaded_skill_context_with_args(skill, args) {
                Ok(rendered) => Ok(CallToolResult::success(vec![ContentBlock::text(rendered)])),
                Err(e) => Ok(CallToolResult::error(vec![ContentBlock::text(format!(
                    "Failed to parse skill arguments: {}",
                    e
                ))])),
            };
        }

        if let Some((parent_skill_name, raw_relative_path)) = skill_name.split_once('/') {
            let relative_path = raw_relative_path.replace('\\', "/");
            if let Some(skill) = skills.iter().find(|s| {
                s.name == parent_skill_name
                    && matches!(s.source_type, SourceType::Skill | SourceType::BuiltinSkill)
            }) {
                let skill_dir = PathBuf::from(&skill.path);
                let canonical_skill_dir = skill_dir
                    .canonicalize()
                    .unwrap_or_else(|_| skill_dir.clone());

                for file_path in &skill.supporting_files {
                    let file_path_buf = Path::new(file_path);
                    let Ok(rel) = file_path_buf.strip_prefix(&skill_dir) else {
                        continue;
                    };
                    if rel.to_string_lossy().replace('\\', "/") != relative_path {
                        continue;
                    }

                    return Ok(match file_path_buf.canonicalize() {
                        Ok(canonical) if canonical.starts_with(&canonical_skill_dir) => {
                            match std::fs::read_to_string(&canonical) {
                                Ok(content) => {
                                    CallToolResult::success(vec![ContentBlock::text(format!(
                                        "# Loaded: {}\n\n{}\n\n---\nFile loaded into context.",
                                        skill_name, content
                                    ))])
                                }
                                Err(e) => CallToolResult::error(vec![ContentBlock::text(format!(
                                    "Failed to read '{}': {}",
                                    skill_name, e
                                ))]),
                            }
                        }
                        Ok(_) => CallToolResult::error(vec![ContentBlock::text(format!(
                            "Refusing to load '{}': resolves outside the skill directory",
                            skill_name
                        ))]),
                        Err(e) => CallToolResult::error(vec![ContentBlock::text(format!(
                            "Failed to resolve '{}': {}",
                            skill_name, e
                        ))]),
                    });
                }

                let available: Vec<String> = skill
                    .supporting_files
                    .iter()
                    .filter_map(|f| {
                        Path::new(f)
                            .strip_prefix(&skill_dir)
                            .ok()
                            .map(|r| r.to_string_lossy().replace('\\', "/"))
                    })
                    .take(10)
                    .collect();

                return Ok(if available.is_empty() {
                    CallToolResult::error(vec![ContentBlock::text(format!(
                        "Skill '{}' has no supporting files.",
                        skill.name
                    ))])
                } else {
                    CallToolResult::error(vec![ContentBlock::text(format!(
                        "File '{}' not found. Available: {}",
                        skill_name,
                        available.join(", ")
                    ))])
                });
            }
        }

        let suggestions: Vec<&str> = skills
            .iter()
            .filter(|s| {
                s.name.to_lowercase().contains(&skill_name.to_lowercase())
                    || skill_name.to_lowercase().contains(&s.name.to_lowercase())
            })
            .take(3)
            .map(|s| s.name.as_str())
            .collect();

        Ok(if suggestions.is_empty() {
            CallToolResult::error(vec![ContentBlock::text(format!(
                "Skill '{}' not found.",
                skill_name
            ))])
        } else {
            CallToolResult::error(vec![ContentBlock::text(format!(
                "Skill '{}' not found. Did you mean: {}?",
                skill_name,
                suggestions.join(", ")
            ))])
        })
    }
}

#[async_trait]
impl McpClientTrait for SkillsClient {
    async fn list_tools(
        &self,
        _session_id: &str,
        _next_cursor: Option<String>,
        _cancellation_token: CancellationToken,
    ) -> Result<ListToolsResult, Error> {
        let load_schema = serde_json::json!({
            "type": "object",
            "required": ["name"],
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Name of the skill to load. Use \"skill-name/path\" to load a supporting file."
                },
                "args": {
                    "type": "string",
                    "description": "Optional arguments to provide when loading the skill."
                }
            }
        });

        let load_tool = Tool::new(
            "load_skill",
            "Load a skill's full content into your context so you can follow its instructions.\n\n\
             Skills are listed in your system instructions. When you need to use one, \
             load it first to get the detailed instructions.\n\n\
             Examples:\n\
             - load_skill(name: \"gdrive\") → Loads the gdrive skill instructions\n\
             - load_skill(name: \"my-skill\", args: \"the arguments for the skill\") → Loads a skill with arguments\n\
             - load_skill(name: \"my-skill/template.md\") → Loads a supporting file"
                .to_string(),
            load_schema.as_object().unwrap().clone(),
        );

        let create_schema = serde_json::json!({
            "type": "object",
            "required": ["name", "description", "content"],
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Kebab-case skill name (lowercase letters, digits, hyphens; max 64 chars)."
                },
                "description": {
                    "type": "string",
                    "description": "What the skill does and when to use it (discovery trigger)."
                },
                "content": {
                    "type": "string",
                    "description": "Markdown body of the skill (instructions below the frontmatter)."
                },
                "global": {
                    "type": "boolean",
                    "description": "If true, create under ~/.agents/skills/. Default false (project .agents/skills/)."
                }
            }
        });

        let create_tool = Tool::new(
            "create_skill",
            "Create a new Agent Skill at the canonical filesystem path.\n\n\
             Project skills go to {working_dir}/.agents/skills/{name}/SKILL.md.\n\
             Global skills go to ~/.agents/skills/{name}/SKILL.md when global=true.\n\n\
             Prefer this tool over writing SKILL.md with file tools. Never write to \
             skills/ or workspace/skills/ — those paths are not discovered.\n\n\
             Examples:\n\
             - create_skill(name: \"code-review\", description: \"…\", content: \"# Code Review\\n…\")\n\
             - create_skill(name: \"my-skill\", description: \"…\", content: \"…\", global: true)"
                .to_string(),
            create_schema.as_object().unwrap().clone(),
        );

        Ok(ListToolsResult {
            tools: vec![load_tool, create_tool],
            next_cursor: None,
            meta: None,
            ..Default::default()
        })
    }

    async fn call_tool(
        &self,
        _ctx: &ToolCallContext,
        name: &str,
        arguments: Option<JsonObject>,
        _cancellation_token: CancellationToken,
    ) -> Result<CallToolResult, Error> {
        match name {
            "load_skill" => self.handle_load_skill(arguments),
            "create_skill" => self.handle_create_skill(arguments),
            _ => Ok(CallToolResult::error(vec![ContentBlock::text(format!(
                "Unknown tool: {}",
                name
            ))])),
        }
    }

    fn get_info(&self) -> Option<&InitializeResult> {
        Some(&self.info)
    }

    fn get_instructions(&self) -> Option<String> {
        let sources = self.discover_skills();
        let mut skills: Vec<&SourceEntry> = sources
            .iter()
            .filter(|s| {
                s.source_type == SourceType::Skill || s.source_type == SourceType::BuiltinSkill
            })
            .collect();
        skills.sort_by(|a, b| (&a.name, &a.path).cmp(&(&b.name, &b.path)));

        let mut instructions = String::from(
            "\n\nYou have these skills at your disposal, when it is clear they can help you solve a problem or you are asked to use them:",
        );
        if skills.is_empty() {
            instructions.push_str("\n(none discovered yet)");
        } else {
            for skill in &skills {
                instructions.push_str(&format!("\n• {} - {}", skill.name, skill.description));
            }
        }

        let working_dir = self.working_dir.to_string_lossy().replace('\\', "/");
        instructions.push_str(&format!(
            "\n\nWhen creating skills, write ONLY to {working_dir}/.agents/skills/<name>/SKILL.md (project) \
             or ~/.agents/skills/<name>/SKILL.md (global). Use the create_skill tool when available. \
             Never use skills/ or workspace/skills/."
        ));

        Some(instructions)
    }

    async fn subscribe(&self) -> mpsc::Receiver<ServerNotification> {
        let (_tx, rx) = mpsc::channel(1);
        rx
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::Arc;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_load_filesystem_skill_without_builtin_skills() {
        let temp_dir = TempDir::new().unwrap();
        let skill_dir = temp_dir.path().join(".goose/skills/my-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: my-skill\ndescription: A test skill\n---\nDo the thing.",
        )
        .unwrap();

        let session = std::sync::Arc::new(crate::session::Session {
            working_dir: temp_dir.path().to_path_buf(),
            ..crate::session::Session::default()
        });
        let client = SkillsClient::new(PlatformExtensionContext {
            extension_manager: None,
            session_manager: Arc::new(crate::session::SessionManager::instance()),
            scheduler: None,
            session: Some(session),
            use_login_shell_path: false,
        })
        .unwrap()
        .with_builtin_skills(false);

        assert!(client
            .discover_skills()
            .iter()
            .all(|skill| skill.source_type != SourceType::BuiltinSkill));

        let ctx = ToolCallContext::new("test".to_string(), None, None);
        let args: JsonObject =
            serde_json::from_value(serde_json::json!({"name": "my-skill"})).unwrap();
        let result = client
            .call_tool(&ctx, "load_skill", Some(args), CancellationToken::new())
            .await
            .unwrap();

        assert!(!result.is_error.unwrap_or(false));
        let text = match &result.content[0] {
            rmcp::model::ContentBlock::Text(t) => &t.text,
            _ => panic!("expected text"),
        };
        assert!(text.contains("my-skill"));
        assert!(text.contains("Do the thing"));
    }

    #[tokio::test]
    async fn test_load_skill_not_found_returns_error() {
        let client = SkillsClient::new(PlatformExtensionContext {
            extension_manager: None,
            session_manager: Arc::new(crate::session::SessionManager::instance()),
            scheduler: None,
            session: None,
            use_login_shell_path: false,
        })
        .unwrap();

        let ctx = ToolCallContext::new("test".to_string(), None, None);
        let args: JsonObject =
            serde_json::from_value(serde_json::json!({"name": "nonexistent"})).unwrap();
        let result = client
            .call_tool(&ctx, "load_skill", Some(args), CancellationToken::new())
            .await
            .unwrap();

        assert!(result.is_error.unwrap_or(false));
    }

    fn test_client(working_dir: PathBuf) -> SkillsClient {
        let session = Arc::new(crate::session::Session {
            working_dir,
            ..crate::session::Session::default()
        });
        SkillsClient::new(PlatformExtensionContext {
            extension_manager: None,
            session_manager: Arc::new(crate::session::SessionManager::instance()),
            scheduler: None,
            session: Some(session),
            use_login_shell_path: false,
        })
        .unwrap()
        .with_builtin_skills(false)
    }

    #[tokio::test]
    async fn get_instructions_includes_agents_skills_path_and_create_skill() {
        let temp_dir = TempDir::new().unwrap();
        let client = test_client(temp_dir.path().to_path_buf());
        let instructions = client.get_instructions().expect("instructions");

        assert!(instructions.contains(".agents/skills"));
        assert!(instructions.contains("create_skill"));
        assert!(instructions.contains("Never use skills/ or workspace/skills/"));
    }

    #[tokio::test]
    async fn test_create_skill_writes_agents_skills_and_is_discoverable() {
        let temp_dir = TempDir::new().unwrap();
        let client = test_client(temp_dir.path().to_path_buf());
        let ctx = ToolCallContext::new("test".to_string(), None, None);
        let args: JsonObject = serde_json::from_value(serde_json::json!({
            "name": "e2e-smoke-test",
            "description": "Greeting skill for smoke tests",
            "content": "# E2E Smoke\n\nSay hello."
        }))
        .unwrap();

        let result = client
            .call_tool(&ctx, "create_skill", Some(args), CancellationToken::new())
            .await
            .unwrap();
        assert!(!result.is_error.unwrap_or(false));

        let skill_md = temp_dir
            .path()
            .join(".agents/skills/e2e-smoke-test/SKILL.md");
        assert!(skill_md.is_file(), "expected {}", skill_md.display());

        let names: Vec<_> = client
            .discover_skills()
            .into_iter()
            .map(|s| s.name)
            .collect();
        assert!(names.iter().any(|n| n == "e2e-smoke-test"));

        let text = match &result.content[0] {
            rmcp::model::ContentBlock::Text(t) => &t.text,
            _ => panic!("expected text"),
        };
        assert!(text.contains(".agents/skills/e2e-smoke-test"));
    }

    #[tokio::test]
    async fn test_list_tools_includes_create_skill() {
        let client = test_client(TempDir::new().unwrap().path().to_path_buf());
        let tools = client
            .list_tools("test", None, CancellationToken::new())
            .await
            .unwrap();
        let names: Vec<_> = tools.tools.iter().map(|t| t.name.to_string()).collect();
        assert!(names.contains(&"load_skill".to_string()));
        assert!(names.contains(&"create_skill".to_string()));
    }
}
