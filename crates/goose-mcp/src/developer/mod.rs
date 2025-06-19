mod editor_models;
mod lang;
mod screen_capture;
mod shell;
mod text_editor;

use anyhow::Result;
use etcetera::{choose_app_strategy, AppStrategy};
use indoc::formatdoc;
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
};
use tokio::sync::mpsc;

use include_dir::{include_dir, Dir};
use mcp_core::{
    handler::{PromptError, ResourceError, ToolError},
    protocol::{JsonRpcMessage, ServerCapabilities},
    resource::Resource,
    tool::Tool,
    Content,
};
use mcp_core::{
    prompt::{Prompt, PromptArgument, PromptTemplate},
    tool::ToolAnnotations,
};
use mcp_server::router::CapabilitiesBuilder;
use mcp_server::Router;

use self::editor_models::{create_editor_model, EditorModel};
use self::screen_capture::{
    capture_screen, create_list_windows_tool, create_screen_capture_tool, list_windows,
    process_image,
};
use self::shell::{create_shell_tool, execute_shell_command, expand_path, is_absolute_path};
use self::text_editor::{create_text_editor_tool, execute_text_editor_command};
use indoc::indoc;
use std::sync::{Arc, Mutex};

use ignore::gitignore::{Gitignore, GitignoreBuilder};

// Embeds the prompts directory to the build
static PROMPTS_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/src/developer/prompts");

/// Loads prompt files from the embedded PROMPTS_DIR and returns a HashMap of prompts.
/// Ensures that each prompt name is unique.
pub fn load_prompt_files() -> HashMap<String, Prompt> {
    let mut prompts = HashMap::new();

    for entry in PROMPTS_DIR.files() {
        let prompt_str = String::from_utf8_lossy(entry.contents()).into_owned();

        let template: PromptTemplate = match serde_json::from_str(&prompt_str) {
            Ok(t) => t,
            Err(e) => {
                eprintln!(
                    "Failed to parse prompt template in {}: {}",
                    entry.path().display(),
                    e
                );
                continue; // Skip invalid prompt file
            }
        };

        let arguments = template
            .arguments
            .into_iter()
            .map(|arg| PromptArgument {
                name: arg.name,
                description: arg.description,
                required: arg.required,
            })
            .collect::<Vec<PromptArgument>>();

        let prompt = Prompt::new(&template.id, Some(&template.template), Some(arguments));

        if prompts.contains_key(&prompt.name) {
            eprintln!("Duplicate prompt name '{}' found. Skipping.", prompt.name);
            continue; // Skip duplicate prompt name
        }

        prompts.insert(prompt.name.clone(), prompt);
    }

    prompts
}

pub struct DeveloperRouter {
    tools: Vec<Tool>,
    prompts: Arc<HashMap<String, Prompt>>,
    instructions: String,
    file_history: Arc<Mutex<HashMap<PathBuf, Vec<String>>>>,
    ignore_patterns: Arc<Gitignore>,
    editor_model: Option<EditorModel>,
}

impl Default for DeveloperRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl DeveloperRouter {
    pub fn new() -> Self {
        // TODO consider rust native search tools, we could use
        // https://docs.rs/ignore/latest/ignore/

        // An editor model is optionally provided, if configured, for fast edit apply
        // it will fall back to norma string replacement if not configured
        //
        // when there is an editor model, the prompts are slightly changed as it takes
        // a load off the main LLM making the tool calls and you get faster more correct applies
        let editor_model = create_editor_model();

        let bash_tool = create_shell_tool();
        let text_editor_tool = create_text_editor_tool();

        let list_windows_tool = create_list_windows_tool();
        let screen_capture_tool = create_screen_capture_tool();

        let image_processor_tool = Tool::new(
            "image_processor",
            indoc! {r#"
                Process an image file from disk. The image will be:
                1. Resized if larger than max width while maintaining aspect ratio
                2. Converted to PNG format
                3. Returned as base64 encoded data

                This allows processing image files for use in the conversation.
            "#},
            json!({
                "type": "object",
                "required": ["path"],
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Absolute path to the image file to process"
                    }
                }
            }),
            Some(ToolAnnotations {
                title: Some("Process Image".to_string()),
                read_only_hint: true,
                destructive_hint: false,
                idempotent_hint: true,
                open_world_hint: false,
            }),
        );

        // Get base instructions and working directory
        let cwd = std::env::current_dir().expect("should have a current working dir");
        let os = std::env::consts::OS;

        let base_instructions = match os {
            "windows" => formatdoc! {r#"
                The developer extension gives you the capabilities to edit code files and run shell commands,
                and can be used to solve a wide range of problems.

                You can use the shell tool to run Windows commands (PowerShell or CMD).
                When using paths, you can use either backslashes or forward slashes.

                Use the shell tool as needed to locate files or interact with the project.

                Your windows/screen tools can be used for visual debugging. You should not use these tools unless
                prompted to, but you can mention they are available if they are relevant.

                operating system: {os}
                current directory: {cwd}

                "#,
                os=os,
                cwd=cwd.to_string_lossy(),
            },
            _ => formatdoc! {r#"
                The developer extension gives you the capabilities to edit code files and run shell commands,
                and can be used to solve a wide range of problems.

            You can use the shell tool to run any command that would work on the relevant operating system.
            Use the shell tool as needed to locate files or interact with the project.

            Your windows/screen tools can be used for visual debugging. You should not use these tools unless
            prompted to, but you can mention they are available if they are relevant.

            operating system: {os}
            current directory: {cwd}

                "#,
                os=os,
                cwd=cwd.to_string_lossy(),
            },
        };

        // choose_app_strategy().config_dir()
        // - macOS/Linux: ~/.config/goose/
        // - Windows:     ~\AppData\Roaming\Block\goose\config\
        // keep previous behavior of expanding ~/.config in case this fails
        let global_hints_path = choose_app_strategy(crate::APP_STRATEGY.clone())
            .map(|strategy| strategy.in_config_dir(".goosehints"))
            .unwrap_or_else(|_| {
                PathBuf::from(shellexpand::tilde("~/.config/goose/.goosehints").to_string())
            });

        // Create the directory if it doesn't exist
        let _ = std::fs::create_dir_all(global_hints_path.parent().unwrap());

        // Check for local hints in current directory
        let local_hints_path = cwd.join(".goosehints");

        // Read global hints if they exist
        let mut hints = String::new();
        if global_hints_path.is_file() {
            if let Ok(global_hints) = std::fs::read_to_string(&global_hints_path) {
                hints.push_str("\n### Global Hints\nThe developer extension includes some global hints that apply to all projects & directories.\n");
                hints.push_str(&global_hints);
            }
        }

        // Read local hints if they exist
        if local_hints_path.is_file() {
            if let Ok(local_hints) = std::fs::read_to_string(&local_hints_path) {
                if !hints.is_empty() {
                    hints.push_str("\n\n");
                }
                hints.push_str("### Project Hints\nThe developer extension includes some hints for working on the project in this directory.\n");
                hints.push_str(&local_hints);
            }
        }

        // Return base instructions directly when no hints are found
        let instructions = if hints.is_empty() {
            base_instructions
        } else {
            format!("{base_instructions}\n{hints}")
        };

        let mut builder = GitignoreBuilder::new(cwd.clone());
        let mut has_ignore_file = false;
        // Initialize ignore patterns
        // - macOS/Linux: ~/.config/goose/
        // - Windows:     ~\AppData\Roaming\Block\goose\config\
        let global_ignore_path = choose_app_strategy(crate::APP_STRATEGY.clone())
            .map(|strategy| strategy.in_config_dir(".gooseignore"))
            .unwrap_or_else(|_| {
                PathBuf::from(shellexpand::tilde("~/.config/goose/.gooseignore").to_string())
            });

        // Create the directory if it doesn't exist
        let _ = std::fs::create_dir_all(global_ignore_path.parent().unwrap());

        // Read global ignores if they exist
        if global_ignore_path.is_file() {
            let _ = builder.add(global_ignore_path);
            has_ignore_file = true;
        }

        // Check for local ignores in current directory
        let local_ignore_path = cwd.join(".gooseignore");

        // Read local ignores if they exist
        if local_ignore_path.is_file() {
            let _ = builder.add(local_ignore_path);
            has_ignore_file = true;
        }

        // Only use default patterns if no .gooseignore files were found
        // If the file is empty, we will not ignore any file
        if !has_ignore_file {
            // Add some sensible defaults
            let _ = builder.add_line(None, "**/.env");
            let _ = builder.add_line(None, "**/.env.*");
            let _ = builder.add_line(None, "**/secrets.*");
        }

        let ignore_patterns = builder.build().expect("Failed to build ignore patterns");

        Self {
            tools: vec![
                bash_tool,
                text_editor_tool,
                list_windows_tool,
                screen_capture_tool,
                image_processor_tool,
            ],
            prompts: Arc::new(load_prompt_files()),
            instructions,
            file_history: Arc::new(Mutex::new(HashMap::new())),
            ignore_patterns: Arc::new(ignore_patterns),
            editor_model,
        }
    }

    // Helper method to check if a path should be ignored
    fn is_ignored(&self, path: &Path) -> bool {
        self.ignore_patterns.matched(path, false).is_ignore()
    }

    // Helper method to resolve a path relative to cwd with platform-specific handling
    fn resolve_path(&self, path_str: &str) -> Result<PathBuf, ToolError> {
        let cwd = std::env::current_dir().expect("should have a current working dir");
        let expanded = expand_path(path_str);
        let path = Path::new(&expanded);

        let suggestion = cwd.join(path);

        match is_absolute_path(&expanded) {
            true => Ok(path.to_path_buf()),
            false => Err(ToolError::InvalidParameters(format!(
                "The path {} is not an absolute path, did you possibly mean {}?",
                path_str,
                suggestion.to_string_lossy(),
            ))),
        }
    }

    // Shell command execution with platform-specific handling
    async fn bash(&self, params: Value) -> Result<Vec<Content>, ToolError> {
        execute_shell_command(params, &self.ignore_patterns).await
    }

    async fn text_editor(&self, params: Value) -> Result<Vec<Content>, ToolError> {
        execute_text_editor_command(
            params,
            &self.ignore_patterns,
            &self.file_history,
            |path_str| self.resolve_path(path_str),
            |path| self.is_ignored(path),
        )
        .await
    }

    async fn list_windows(&self, params: Value) -> Result<Vec<Content>, ToolError> {
        list_windows(params).await
    }

    async fn image_processor(&self, params: Value) -> Result<Vec<Content>, ToolError> {
        process_image(params, &self.ignore_patterns, |path_str| {
            self.resolve_path(path_str)
        })
        .await
    }

    async fn screen_capture(&self, params: Value) -> Result<Vec<Content>, ToolError> {
        capture_screen(params).await
    }
}

impl Router for DeveloperRouter {
    fn name(&self) -> String {
        "developer".to_string()
    }

    fn instructions(&self) -> String {
        self.instructions.clone()
    }

    fn capabilities(&self) -> ServerCapabilities {
        CapabilitiesBuilder::new()
            .with_tools(false)
            .with_prompts(false)
            .build()
    }

    fn list_tools(&self) -> Vec<Tool> {
        self.tools.clone()
    }

    fn call_tool(
        &self,
        tool_name: &str,
        arguments: Value,
        _notifier: mpsc::Sender<JsonRpcMessage>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Content>, ToolError>> + Send + 'static>> {
        let this = self.clone();
        let tool_name = tool_name.to_string();
        Box::pin(async move {
            match tool_name.as_str() {
                "shell" => this.bash(arguments).await,
                "text_editor" => this.text_editor(arguments).await,
                "list_windows" => this.list_windows(arguments).await,
                "screen_capture" => this.screen_capture(arguments).await,
                "image_processor" => this.image_processor(arguments).await,
                _ => Err(ToolError::NotFound(format!("Tool {} not found", tool_name))),
            }
        })
    }

    // TODO see if we can make it easy to skip implementing these
    fn list_resources(&self) -> Vec<Resource> {
        Vec::new()
    }

    fn read_resource(
        &self,
        _uri: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, ResourceError>> + Send + 'static>> {
        Box::pin(async move { Ok("".to_string()) })
    }

    fn list_prompts(&self) -> Vec<Prompt> {
        self.prompts.values().cloned().collect()
    }

    fn get_prompt(
        &self,
        prompt_name: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, PromptError>> + Send + 'static>> {
        let prompt_name = prompt_name.trim().to_owned();

        // Validate prompt name is not empty
        if prompt_name.is_empty() {
            return Box::pin(async move {
                Err(PromptError::InvalidParameters(
                    "Prompt name cannot be empty".to_string(),
                ))
            });
        }

        let prompts = Arc::clone(&self.prompts);

        Box::pin(async move {
            match prompts.get(&prompt_name) {
                Some(prompt) => Ok(prompt.description.clone().unwrap_or_default()),
                None => Err(PromptError::NotFound(format!(
                    "Prompt '{prompt_name}' not found"
                ))),
            }
        })
    }
}

impl Clone for DeveloperRouter {
    fn clone(&self) -> Self {
        Self {
            tools: self.tools.clone(),
            prompts: Arc::clone(&self.prompts),
            instructions: self.instructions.clone(),
            file_history: Arc::clone(&self.file_history),
            ignore_patterns: Arc::clone(&self.ignore_patterns),
            editor_model: self.editor_model.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mcp_core::role::Role;
    use serde_json::json;
    use serial_test::serial;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    #[serial]
    fn test_global_goosehints() {
        // if ~/.config/goose/.goosehints exists, it should be included in the instructions
        // copy the existing global hints file to a .bak file
        let global_hints_path =
            PathBuf::from(shellexpand::tilde("~/.config/goose/.goosehints").to_string());
        let global_hints_bak_path =
            PathBuf::from(shellexpand::tilde("~/.config/goose/.goosehints.bak").to_string());
        let mut globalhints_existed = false;

        if global_hints_path.is_file() {
            globalhints_existed = true;
            fs::copy(&global_hints_path, &global_hints_bak_path).unwrap();
        }

        fs::write(&global_hints_path, "These are my global goose hints.").unwrap();

        let dir = TempDir::new().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let router = DeveloperRouter::new();
        let instructions = router.instructions();

        assert!(instructions.contains("### Global Hints"));
        assert!(instructions.contains("my global goose hints."));

        // restore backup if globalhints previously existed
        if globalhints_existed {
            fs::copy(&global_hints_bak_path, &global_hints_path).unwrap();
            fs::remove_file(&global_hints_bak_path).unwrap();
        }
    }

    #[test]
    #[serial]
    fn test_goosehints_when_present() {
        let dir = TempDir::new().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        fs::write(".goosehints", "Test hint content").unwrap();
        let router = DeveloperRouter::new();
        let instructions = router.instructions();

        assert!(instructions.contains("Test hint content"));
    }

    #[test]
    #[serial]
    fn test_goosehints_when_missing() {
        let dir = TempDir::new().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let router = DeveloperRouter::new();
        let instructions = router.instructions();

        assert!(!instructions.contains("Project Hints"));
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_write_and_view_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let router = DeveloperRouter::new();
        let file_path = temp_dir.path().join("test.txt");
        let file_path_str = file_path.to_str().unwrap();

        let (_tx, _rx) = mpsc::channel(1);

        // Create a new file
        router
            .call_tool(
                "text_editor",
                json!({
                    "command": "write",
                    "path": file_path_str,
                    "file_text": "Hello, world!"
                }),
                _tx.clone(),
            )
            .await
            .unwrap();

        // View the file
        let view_result = router
            .call_tool(
                "text_editor",
                json!({
                    "command": "view",
                    "path": file_path_str
                }),
                _tx.clone(),
            )
            .await
            .unwrap();

        assert!(!view_result.is_empty());
        let text = view_result
            .iter()
            .find(|c| {
                c.audience()
                    .is_some_and(|roles| roles.contains(&Role::User))
            })
            .unwrap()
            .as_text()
            .unwrap();
        assert!(text.contains("Hello, world!"));

        temp_dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_str_replace() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let router = DeveloperRouter::new();
        let file_path = temp_dir.path().join("test.txt");
        let file_path_str = file_path.to_str().unwrap();

        let (_tx, _rx) = mpsc::channel(1);

        // Create a new file
        router
            .call_tool(
                "text_editor",
                json!({
                    "command": "write",
                    "path": file_path_str,
                    "file_text": "Hello, world!"
                }),
                _tx.clone(),
            )
            .await
            .unwrap();

        // Replace string
        let replace_result = router
            .call_tool(
                "text_editor",
                json!({
                    "command": "str_replace",
                    "path": file_path_str,
                    "old_str": "world",
                    "new_str": "Rust"
                }),
                _tx.clone(),
            )
            .await
            .unwrap();

        let text = replace_result
            .iter()
            .find(|c| {
                c.audience()
                    .is_some_and(|roles| roles.contains(&Role::Assistant))
            })
            .unwrap()
            .as_text()
            .unwrap();

        assert!(text.contains("has been edited, and the section now reads"));

        // View the file to verify the change
        let view_result = router
            .call_tool(
                "text_editor",
                json!({
                    "command": "view",
                    "path": file_path_str
                }),
                _tx.clone(),
            )
            .await
            .unwrap();

        let text = view_result
            .iter()
            .find(|c| {
                c.audience()
                    .is_some_and(|roles| roles.contains(&Role::User))
            })
            .unwrap()
            .as_text()
            .unwrap();
        assert!(text.contains("Hello, Rust!"));

        temp_dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_undo_edit() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let router = DeveloperRouter::new();
        let file_path = temp_dir.path().join("test.txt");
        let file_path_str = file_path.to_str().unwrap();

        let (_tx, _rx) = mpsc::channel(1);

        // Create a new file
        router
            .call_tool(
                "text_editor",
                json!({
                    "command": "write",
                    "path": file_path_str,
                    "file_text": "First line"
                }),
                _tx.clone(),
            )
            .await
            .unwrap();

        // Replace string
        router
            .call_tool(
                "text_editor",
                json!({
                    "command": "str_replace",
                    "path": file_path_str,
                    "old_str": "First line",
                    "new_str": "Second line"
                }),
                _tx.clone(),
            )
            .await
            .unwrap();

        // Undo the edit
        let undo_result = router
            .call_tool(
                "text_editor",
                json!({
                    "command": "undo_edit",
                    "path": file_path_str
                }),
                _tx.clone(),
            )
            .await
            .unwrap();

        let text = undo_result.first().unwrap().as_text().unwrap();
        assert!(text.contains("Undid the last edit"));

        // View the file to verify the undo
        let view_result = router
            .call_tool(
                "text_editor",
                json!({
                    "command": "view",
                    "path": file_path_str
                }),
                _tx.clone(),
            )
            .await
            .unwrap();

        let text = view_result
            .iter()
            .find(|c| {
                c.audience()
                    .is_some_and(|roles| roles.contains(&Role::User))
            })
            .unwrap()
            .as_text()
            .unwrap();
        assert!(text.contains("First line"));

        temp_dir.close().unwrap();
    }

    // Test GooseIgnore pattern matching
    #[tokio::test]
    #[serial]
    async fn test_goose_ignore_basic_patterns() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Create a DeveloperRouter with custom ignore patterns
        let mut builder = GitignoreBuilder::new(temp_dir.path().to_path_buf());
        builder.add_line(None, "secret.txt").unwrap();
        builder.add_line(None, "*.env").unwrap();
        let ignore_patterns = builder.build().unwrap();

        let router = DeveloperRouter {
            tools: vec![],
            prompts: Arc::new(HashMap::new()),
            instructions: String::new(),
            file_history: Arc::new(Mutex::new(HashMap::new())),
            ignore_patterns: Arc::new(ignore_patterns),
            editor_model: None,
        };

        // Test basic file matching
        assert!(
            router.is_ignored(Path::new("secret.txt")),
            "secret.txt should be ignored"
        );
        assert!(
            router.is_ignored(Path::new("./secret.txt")),
            "./secret.txt should be ignored"
        );
        assert!(
            !router.is_ignored(Path::new("not_secret.txt")),
            "not_secret.txt should not be ignored"
        );

        // Test pattern matching
        assert!(
            router.is_ignored(Path::new("test.env")),
            "*.env pattern should match test.env"
        );
        assert!(
            router.is_ignored(Path::new("./test.env")),
            "*.env pattern should match ./test.env"
        );
        assert!(
            !router.is_ignored(Path::new("test.txt")),
            "*.env pattern should not match test.txt"
        );

        temp_dir.close().unwrap();
    }
}
