use async_trait::async_trait;
use etcetera::{choose_app_strategy, AppStrategy};
use indoc::formatdoc;
use mcp_core::{
    handler::{PromptError, ResourceError, ToolError},
    protocol::ServerCapabilities,
    tool::ToolCall,
};
use mcp_server::router::CapabilitiesBuilder;
use mcp_server::Router;
use rmcp::model::{Content, JsonRpcMessage, Prompt, Resource, Tool, ToolAnnotations};
use rmcp::object;
use serde_json::Value;
use std::{
    collections::HashMap,
    fs,
    future::Future,
    io::{self, Read, Write},
    path::PathBuf,
    pin::Pin,
};
use tokio::sync::mpsc;

// MemoryRouter implementation
#[derive(Clone)]
pub struct MemoryRouter {
    tools: Vec<Tool>,
    instructions: String,
    global_memory_dir: PathBuf,
    local_memory_dir: PathBuf,
}

impl Default for MemoryRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryRouter {
    pub fn new() -> Self {
        let remember_memory = Tool::new(
            "remember_memory",
            "Stores a memory with optional tags in a specified category",
            object!({
                "type": "object",
                "properties": {
                    "category": {"type": "string"},
                    "data": {"type": "string"},
                    "tags": {"type": "array", "items": {"type": "string"}},
                    "is_global": {"type": "boolean"}
                },
                "required": ["category", "data", "is_global"]
            }),
        )
        .annotate(ToolAnnotations {
            title: Some("Remember Memory".to_string()),
            read_only_hint: Some(false),
            destructive_hint: Some(false),
            idempotent_hint: Some(true),
            open_world_hint: Some(false),
        });

        let retrieve_memories = Tool::new(
            "retrieve_memories",
            "Retrieves all memories from a specified category",
            object!({
                "type": "object",
                "properties": {
                    "category": {"type": "string"},
                    "is_global": {"type": "boolean"}
                },
                "required": ["category", "is_global"]
            }),
        )
        .annotate(ToolAnnotations {
            title: Some("Retrieve Memory".to_string()),
            read_only_hint: Some(true),
            destructive_hint: Some(false),
            idempotent_hint: Some(false),
            open_world_hint: Some(false),
        });

        let remove_memory_category = Tool::new(
            "remove_memory_category",
            "Removes all memories within a specified category",
            object!({
                "type": "object",
                "properties": {
                    "category": {"type": "string"},
                    "is_global": {"type": "boolean"}
                },
                "required": ["category", "is_global"]
            }),
        )
        .annotate(ToolAnnotations {
            title: Some("Remove Memory Category".to_string()),
            read_only_hint: Some(false),
            destructive_hint: Some(true),
            idempotent_hint: Some(false),
            open_world_hint: Some(false),
        });

        let remove_specific_memory = Tool::new(
            "remove_specific_memory",
            "Removes a specific memory within a specified category",
            object!({
                "type": "object",
                "properties": {
                    "category": {"type": "string"},
                    "memory_content": {"type": "string"},
                    "is_global": {"type": "boolean"}
                },
                "required": ["category", "memory_content", "is_global"]
            }),
        )
        .annotate(ToolAnnotations {
            title: Some("Remove Specific Memory".to_string()),
            read_only_hint: Some(false),
            destructive_hint: Some(true),
            idempotent_hint: Some(false),
            open_world_hint: Some(false),
        });

        let instructions = formatdoc! {r#"
             This extension allows storage and retrieval of categorized information with tagging support. It's designed to help
             manage important information across sessions in a systematic and organized manner.
             Capabilities:
             1. Store information in categories with optional tags for context-based retrieval.
             2. Search memories by content or specific tags to find relevant information.
             3. List all available memory categories for easy navigation.
             4. Remove entire categories of memories when they are no longer needed.
             When to call memory tools:
             - These are examples where the assistant should proactively call the memory tool because the user is providing recurring preferences, project details, or workflow habits that they may expect to be remembered.
             - Preferred Development Tools & Conventions
             - User-specific data (e.g., name, preferences)
             - Project-related configurations
             - Workflow descriptions
             - Other critical settings
             Interaction Protocol:
             When important information is identified, such as:
             - User-specific data (e.g., name, preferences)
             - Project-related configurations
             - Workflow descriptions
             - Other critical settings
             The protocol is:
             1. Identify the critical piece of information.
             2. Ask the user if they'd like to store it for later reference.
             3. Upon agreement:
                - Suggest a relevant category like "personal" for user data or "development" for project preferences.
                - Inquire about any specific tags they want to apply for easier lookup.
                - Confirm the desired storage location:
                  - Local storage (.goose/memory) for project-specific details.
                  - Global storage (~/.config/goose/memory) for user-wide data.
                - Use the remember_memory tool to store the information.
                  - `remember_memory(category, data, tags, is_global)`
             Keywords that trigger memory tools:
             - "remember"
             - "forget"
             - "memory"
             - "save"
             - "save memory"
             - "remove memory"
             - "clear memory"
             - "search memory"
             - "find memory"
             Suggest the user to use memory tools when:
             - When the user mentions a keyword that triggers a memory tool
             - When the user performs a routine task
             - When the user executes a command and would benefit from remembering the exact command
             Example Interaction for Storing Information:
             User: "For this project, we use black for code formatting"
             Assistant: "You've mentioned a development preference. Would you like to remember this for future conversations?
             User: "Yes, please."
             Assistant: "I'll store this in the 'development' category. Any specific tags to add? Suggestions: #formatting
             #tools"
             User: "Yes, use those tags."
             Assistant: "Shall I store this locally for this project only, or globally for all projects?"
             User: "Locally, please."
             Assistant: *Stores the information under category="development", tags="formatting tools", scope="local"*
             Another Example Interaction for Storing Information:
             User: "Remember the gh command to view github comments"
             Assistant: "Shall I store this locally for this project only, or globally for all projects?"
             User: "Globally, please."
             Assistant: *Stores the gh command under category="github", tags="comments", scope="global"*
             Example Interaction suggesting memory tools:
             User: "I'm using the gh command to view github comments"
             Assistant: "You've mentioned a command. Would you like to remember this for future conversations?
             User: "Yes, please."
             Assistant: "I'll store this in the 'github' category. Any specific tags to add? Suggestions: #comments #gh"
             Retrieving Memories:
             To access stored information, utilize the memory retrieval protocols:
             - **Search by Category**:
               - Provides all memories within the specified context.
               - Use: `retrieve_memories(category="development", is_global=False)`
               - Note: If you want to retrieve all local memories, use `retrieve_memories(category="*", is_global=False)`
               - Note: If you want to retrieve all global memories, use `retrieve_memories(category="*", is_global=True)`
             - **Filter by Tags**:
               - Enables targeted retrieval based on specific tags.
               - Use: Provide tag filters to refine search.
            To remove a memory, use the following protocol:
            - **Remove by Category**:
              - Removes all memories within the specified category.
              - Use: `remove_memory_category(category="development", is_global=False)`
              - Note: If you want to remove all local memories, use `remove_memory_category(category="*", is_global=False)`
              - Note: If you want to remove all global memories, use `remove_memory_category(category="*", is_global=True)`
            The Protocol is:
             1. Confirm what kind of information the user seeks by category or keyword.
             2. Suggest categories or relevant tags based on the user's request.
             3. Use the retrieve function to access relevant memory entries.
             4. Present a summary of findings, offering detailed exploration upon request.
             Example Interaction for Retrieving Information:
             User: "What configuration do we use for code formatting?"
             Assistant: "Let me check the 'development' category for any related memories. Searching using #formatting tag."
             Assistant: *Executes retrieval: `retrieve_memories(category="development", is_global=False)`*
             Assistant: "We have 'black' configured for code formatting, specific to this project. Would you like further
             details?"
             Memory Overview:
             - Categories can include a wide range of topics, structured to keep information grouped logically.
             - Tags enable quick filtering and identification of specific entries.
             Operational Guidelines:
             - Always confirm with the user before saving information.
             - Propose suitable categories and tag suggestions.
             - Discuss storage scope thoroughly to align with user needs.
             - Acknowledge the user about what is stored and where, for transparency and ease of future retrieval.
            "#};

        // Check for .goose/memory in current directory
        let local_memory_dir = std::env::var("GOOSE_WORKING_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::current_dir().unwrap())
            .join(".goose")
            .join("memory");

        // choose_app_strategy().config_dir()
        // - macOS/Linux: ~/.config/goose/memory/
        // - Windows:     ~\AppData\Roaming\Block\goose\config\memory
        // if it fails, fall back to `.config/goose/memory` (relative to the current dir)
        let global_memory_dir = choose_app_strategy(crate::APP_STRATEGY.clone())
            .map(|strategy| strategy.in_config_dir("memory"))
            .unwrap_or_else(|_| PathBuf::from(".config/goose/memory"));

        let mut memory_router = Self {
            tools: vec![
                remember_memory,
                retrieve_memories,
                remove_memory_category,
                remove_specific_memory,
            ],
            instructions: instructions.clone(),
            global_memory_dir,
            local_memory_dir,
        };

        let retrieved_global_memories = memory_router.retrieve_all(true);
        let retrieved_local_memories = memory_router.retrieve_all(false);

        let mut updated_instructions = instructions;

        let memories_follow_up_instructions = formatdoc! {r#"
            **Here are the user's currently saved memories:**
            Please keep this information in mind when answering future questions.
            Do not bring up memories unless relevant.
            Note: if the user has not saved any memories, this section will be empty.
            Note: if the user removes a memory that was previously loaded into the system, please remove it from the system instructions.
            "#};

        updated_instructions.push_str("\n\n");
        updated_instructions.push_str(&memories_follow_up_instructions);

        if let Ok(global_memories) = retrieved_global_memories {
            if !global_memories.is_empty() {
                updated_instructions.push_str("\n\nGlobal Memories:\n");
                for (category, memories) in global_memories {
                    updated_instructions.push_str(&format!("\nCategory: {}\n", category));
                    for memory in memories {
                        updated_instructions.push_str(&format!("- {}\n", memory));
                    }
                }
            }
        }

        if let Ok(local_memories) = retrieved_local_memories {
            if !local_memories.is_empty() {
                updated_instructions.push_str("\n\nLocal Memories:\n");
                for (category, memories) in local_memories {
                    updated_instructions.push_str(&format!("\nCategory: {}\n", category));
                    for memory in memories {
                        updated_instructions.push_str(&format!("- {}\n", memory));
                    }
                }
            }
        }

        memory_router.set_instructions(updated_instructions);

        memory_router
    }

    // Add a setter method for instructions
    pub fn set_instructions(&mut self, new_instructions: String) {
        self.instructions = new_instructions;
    }

    pub fn get_instructions(&self) -> &str {
        &self.instructions
    }

    fn get_memory_file(&self, category: &str, is_global: bool) -> PathBuf {
        // Defaults to local memory if no is_global flag is provided
        let base_dir = if is_global {
            &self.global_memory_dir
        } else {
            &self.local_memory_dir
        };
        base_dir.join(format!("{}.txt", category))
    }

    pub fn retrieve_all(&self, is_global: bool) -> io::Result<HashMap<String, Vec<String>>> {
        let base_dir = if is_global {
            &self.global_memory_dir
        } else {
            &self.local_memory_dir
        };
        let mut memories = HashMap::new();
        if base_dir.exists() {
            for entry in fs::read_dir(base_dir)? {
                let entry = entry?;
                if entry.file_type()?.is_file() {
                    let category = entry.file_name().to_string_lossy().replace(".txt", "");
                    let category_memories = self.retrieve(&category, is_global)?;
                    memories.insert(
                        category,
                        category_memories.into_iter().flat_map(|(_, v)| v).collect(),
                    );
                }
            }
        }
        Ok(memories)
    }

    pub fn remember(
        &self,
        _context: &str,
        category: &str,
        data: &str,
        tags: &[&str],
        is_global: bool,
    ) -> io::Result<()> {
        let memory_file_path = self.get_memory_file(category, is_global);

        if let Some(parent) = memory_file_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut file = fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&memory_file_path)?;
        if !tags.is_empty() {
            writeln!(file, "# {}", tags.join(" "))?;
        }
        writeln!(file, "{}\n", data)?;

        Ok(())
    }

    pub fn retrieve(
        &self,
        category: &str,
        is_global: bool,
    ) -> io::Result<HashMap<String, Vec<String>>> {
        let memory_file_path = self.get_memory_file(category, is_global);
        if !memory_file_path.exists() {
            return Ok(HashMap::new());
        }

        let mut file = fs::File::open(memory_file_path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;

        let mut memories = HashMap::new();
        for entry in content.split("\n\n") {
            let mut lines = entry.lines();
            if let Some(first_line) = lines.next() {
                if let Some(stripped) = first_line.strip_prefix('#') {
                    let tags = stripped
                        .split_whitespace()
                        .map(String::from)
                        .collect::<Vec<_>>();
                    memories.insert(tags.join(" "), lines.map(String::from).collect());
                } else {
                    let entry_data: Vec<String> = std::iter::once(first_line.to_string())
                        .chain(lines.map(String::from))
                        .collect();
                    memories
                        .entry("untagged".to_string())
                        .or_insert_with(Vec::new)
                        .extend(entry_data);
                }
            }
        }

        Ok(memories)
    }

    pub fn remove_specific_memory(
        &self,
        category: &str,
        memory_content: &str,
        is_global: bool,
    ) -> io::Result<()> {
        let memory_file_path = self.get_memory_file(category, is_global);
        if !memory_file_path.exists() {
            return Ok(());
        }

        let mut file = fs::File::open(&memory_file_path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;

        let memories: Vec<&str> = content.split("\n\n").collect();
        let new_content: Vec<String> = memories
            .into_iter()
            .filter(|entry| !entry.contains(memory_content))
            .map(|s| s.to_string())
            .collect();

        fs::write(memory_file_path, new_content.join("\n\n"))?;

        Ok(())
    }

    pub fn clear_memory(&self, category: &str, is_global: bool) -> io::Result<()> {
        let memory_file_path = self.get_memory_file(category, is_global);
        if memory_file_path.exists() {
            fs::remove_file(memory_file_path)?;
        }

        Ok(())
    }

    pub fn clear_all_global_or_local_memories(&self, is_global: bool) -> io::Result<()> {
        let base_dir = if is_global {
            &self.global_memory_dir
        } else {
            &self.local_memory_dir
        };
        if base_dir.exists() {
            fs::remove_dir_all(base_dir)?;
        }
        Ok(())
    }

    async fn execute_tool_call(&self, tool_call: ToolCall) -> Result<String, io::Error> {
        match tool_call.name.as_str() {
            "remember_memory" => {
                let args = MemoryArgs::from_value(&tool_call.arguments)?;
                let data = args.data.filter(|d| !d.is_empty()).ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "Data must exist when remembering a memory",
                    )
                })?;
                self.remember("context", args.category, data, &args.tags, args.is_global)?;
                Ok(format!("Stored memory in category: {}", args.category))
            }
            "retrieve_memories" => {
                let args = MemoryArgs::from_value(&tool_call.arguments)?;
                let memories = if args.category == "*" {
                    self.retrieve_all(args.is_global)?
                } else {
                    self.retrieve(args.category, args.is_global)?
                };
                Ok(format!("Retrieved memories: {:?}", memories))
            }
            "remove_memory_category" => {
                let args = MemoryArgs::from_value(&tool_call.arguments)?;
                if args.category == "*" {
                    self.clear_all_global_or_local_memories(args.is_global)?;
                    Ok(format!(
                        "Cleared all memory {} categories",
                        if args.is_global { "global" } else { "local" }
                    ))
                } else {
                    self.clear_memory(args.category, args.is_global)?;
                    Ok(format!("Cleared memories in category: {}", args.category))
                }
            }
            "remove_specific_memory" => {
                let args = MemoryArgs::from_value(&tool_call.arguments)?;
                let memory_content = tool_call.arguments["memory_content"].as_str().unwrap();
                self.remove_specific_memory(args.category, memory_content, args.is_global)?;
                Ok(format!(
                    "Removed specific memory from category: {}",
                    args.category
                ))
            }
            _ => Err(io::Error::new(io::ErrorKind::InvalidInput, "Unknown tool")),
        }
    }
}

#[async_trait]
impl Router for MemoryRouter {
    fn name(&self) -> String {
        "memory".to_string()
    }

    fn instructions(&self) -> String {
        self.instructions.clone()
    }

    fn capabilities(&self) -> ServerCapabilities {
        CapabilitiesBuilder::new().with_tools(false).build()
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
            let tool_call = ToolCall {
                name: tool_name,
                arguments,
            };
            match this.execute_tool_call(tool_call).await {
                Ok(result) => Ok(vec![Content::text(result)]),
                Err(err) => Err(ToolError::ExecutionError(err.to_string())),
            }
        })
    }

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
        vec![]
    }

    fn get_prompt(
        &self,
        prompt_name: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, PromptError>> + Send + 'static>> {
        let prompt_name = prompt_name.to_string();
        Box::pin(async move {
            Err(PromptError::NotFound(format!(
                "Prompt {} not found",
                prompt_name
            )))
        })
    }
}

#[derive(Debug)]
struct MemoryArgs<'a> {
    category: &'a str,
    data: Option<&'a str>,
    tags: Vec<&'a str>,
    is_global: bool,
}

impl<'a> MemoryArgs<'a> {
    // Category is required, data is optional, tags are optional, is_global is optional
    fn from_value(args: &'a Value) -> Result<Self, io::Error> {
        let category = args["category"].as_str().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "Category must be a string")
        })?;

        if category.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Category must be a string",
            ));
        }

        let data = args.get("data").and_then(|d| d.as_str());

        let tags = match &args["tags"] {
            Value::Array(arr) => arr.iter().filter_map(|v| v.as_str()).collect(),
            Value::String(s) => vec![s.as_str()],
            _ => Vec::new(),
        };

        let is_global = match &args.get("is_global") {
            // Default to false if no is_global flag is provided
            Some(Value::Bool(b)) => *b,
            Some(Value::String(s)) => s.to_lowercase() == "true",
            None => false,
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "is_global must be a boolean or string 'true'/'false'",
                ))
            }
        };

        Ok(Self {
            category,
            data,
            tags,
            is_global,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_lazy_directory_creation() {
        let temp_dir = tempdir().unwrap();
        let memory_base = temp_dir.path().join("test_memory");

        let router = MemoryRouter {
            tools: vec![],
            instructions: String::new(),
            global_memory_dir: memory_base.join("global"),
            local_memory_dir: memory_base.join("local"),
        };

        assert!(!router.global_memory_dir.exists());
        assert!(!router.local_memory_dir.exists());

        router
            .remember(
                "test_context",
                "test_category",
                "test_data",
                &["tag1"],
                false,
            )
            .unwrap();

        assert!(router.local_memory_dir.exists());
        assert!(!router.global_memory_dir.exists());

        router
            .remember(
                "test_context",
                "global_category",
                "global_data",
                &["global_tag"],
                true,
            )
            .unwrap();

        assert!(router.global_memory_dir.exists());
    }

    #[test]
    fn test_clear_nonexistent_directories() {
        let temp_dir = tempdir().unwrap();
        let memory_base = temp_dir.path().join("nonexistent_memory");

        let router = MemoryRouter {
            tools: vec![],
            instructions: String::new(),
            global_memory_dir: memory_base.join("global"),
            local_memory_dir: memory_base.join("local"),
        };

        assert!(router.clear_all_global_or_local_memories(false).is_ok());
        assert!(router.clear_all_global_or_local_memories(true).is_ok());
    }

    #[test]
    fn test_remember_retrieve_clear_workflow() {
        let temp_dir = tempdir().unwrap();
        let memory_base = temp_dir.path().join("workflow_test");

        let router = MemoryRouter {
            tools: vec![],
            instructions: String::new(),
            global_memory_dir: memory_base.join("global"),
            local_memory_dir: memory_base.join("local"),
        };

        router
            .remember(
                "context",
                "test_category",
                "test_data_content",
                &["test_tag"],
                false,
            )
            .unwrap();

        let memories = router.retrieve("test_category", false).unwrap();
        assert!(!memories.is_empty());

        let has_content = memories.values().any(|v| {
            v.iter()
                .any(|content| content.contains("test_data_content"))
        });
        assert!(has_content);

        router.clear_memory("test_category", false).unwrap();

        let memories_after_clear = router.retrieve("test_category", false).unwrap();
        assert!(memories_after_clear.is_empty());
    }

    #[test]
    fn test_directory_creation_on_write() {
        let temp_dir = tempdir().unwrap();
        let memory_base = temp_dir.path().join("write_test");

        let router = MemoryRouter {
            tools: vec![],
            instructions: String::new(),
            global_memory_dir: memory_base.join("global"),
            local_memory_dir: memory_base.join("local"),
        };

        assert!(!router.local_memory_dir.exists());

        router
            .remember("context", "category", "data", &[], false)
            .unwrap();

        assert!(router.local_memory_dir.exists());
        assert!(router.local_memory_dir.join("category.txt").exists());
    }

    #[test]
    fn test_remove_specific_memory() {
        let temp_dir = tempdir().unwrap();
        let memory_base = temp_dir.path().join("remove_test");

        let router = MemoryRouter {
            tools: vec![],
            instructions: String::new(),
            global_memory_dir: memory_base.join("global"),
            local_memory_dir: memory_base.join("local"),
        };

        router
            .remember("context", "category", "keep_this", &[], false)
            .unwrap();
        router
            .remember("context", "category", "remove_this", &[], false)
            .unwrap();

        let memories = router.retrieve("category", false).unwrap();
        assert_eq!(memories.len(), 1);

        router
            .remove_specific_memory("category", "remove_this", false)
            .unwrap();

        let memories_after = router.retrieve("category", false).unwrap();
        let has_removed = memories_after
            .values()
            .any(|v| v.iter().any(|content| content.contains("remove_this")));
        assert!(!has_removed);

        let has_kept = memories_after
            .values()
            .any(|v| v.iter().any(|content| content.contains("keep_this")));
        assert!(has_kept);
    }
}
