use etcetera::{choose_app_strategy, AppStrategy};
use indoc::formatdoc;
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{
        CallToolResult, Content, ErrorCode, ErrorData, Implementation, Meta, ServerCapabilities,
        ServerInfo,
    },
    schemars::JsonSchema,
    service::RequestContext,
    tool, tool_handler, tool_router, RoleServer, ServerHandler,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    io::{self, Read, Write},
    path::PathBuf,
};

const WORKING_DIR_HEADER: &str = "agent-working-dir";

fn extract_working_dir_from_meta(meta: &Meta) -> Option<PathBuf> {
    meta.0
        .get(WORKING_DIR_HEADER)
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
}

/// Parameters for the remember_memory tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RememberMemoryParams {
    /// The category to store the memory in
    pub category: String,
    /// The data to remember
    pub data: String,
    /// Optional tags for the memory
    #[serde(default)]
    pub tags: Vec<String>,
    /// Whether to store globally or locally
    pub is_global: bool,
}

/// Parameters for the retrieve_memories tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RetrieveMemoriesParams {
    /// The category to retrieve memories from (use "*" for all)
    pub category: String,
    /// Whether to retrieve from global or local storage
    pub is_global: bool,
}

/// Parameters for the remove_memory_category tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RemoveMemoryCategoryParams {
    /// The category to remove (use "*" for all)
    pub category: String,
    /// Whether to remove from global or local storage
    pub is_global: bool,
}

/// Parameters for the remove_specific_memory tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RemoveSpecificMemoryParams {
    /// The category containing the memory
    pub category: String,
    /// The content of the memory to remove
    pub memory_content: String,
    /// Whether to remove from global or local storage
    pub is_global: bool,
}

/// Memory MCP Server using official RMCP SDK
#[derive(Clone)]
pub struct MemoryServer {
    tool_router: ToolRouter<Self>,
    instructions: String,
    global_memory_dir: PathBuf,
}

impl Default for MemoryServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router(router = tool_router)]
impl MemoryServer {
    pub fn new() -> Self {
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

        let global_memory_dir = choose_app_strategy(crate::APP_STRATEGY.clone())
            .map(|strategy| strategy.in_config_dir("memory"))
            .unwrap_or_else(|_| PathBuf::from(".config/goose/memory"));

        let mut memory_router = Self {
            tool_router: Self::tool_router(),
            instructions: instructions.clone(),
            global_memory_dir,
        };

        let retrieved_global_memories = memory_router.retrieve_all(true, None);

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

    fn get_memory_file(
        &self,
        category: &str,
        is_global: bool,
        working_dir: Option<&PathBuf>,
    ) -> PathBuf {
        let base_dir = if is_global {
            self.global_memory_dir.clone()
        } else {
            let local_base = working_dir
                .cloned()
                .or_else(|| std::env::current_dir().ok())
                .unwrap_or_else(|| PathBuf::from("."));
            local_base.join(".goose").join("memory")
        };
        base_dir.join(format!("{}.txt", category))
    }

    pub fn retrieve_all(
        &self,
        is_global: bool,
        working_dir: Option<&PathBuf>,
    ) -> io::Result<HashMap<String, Vec<String>>> {
        let base_dir = if is_global {
            self.global_memory_dir.clone()
        } else {
            let local_base = working_dir
                .cloned()
                .or_else(|| std::env::current_dir().ok())
                .unwrap_or_else(|| PathBuf::from("."));
            local_base.join(".goose").join("memory")
        };
        let mut memories = HashMap::new();
        if base_dir.exists() {
            for entry in fs::read_dir(&base_dir)? {
                let entry = entry?;
                if entry.file_type()?.is_file() {
                    let category = entry.file_name().to_string_lossy().replace(".txt", "");
                    let category_memories = self.retrieve(&category, is_global, working_dir)?;
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
        working_dir: Option<&PathBuf>,
    ) -> io::Result<()> {
        let memory_file_path = self.get_memory_file(category, is_global, working_dir);

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
        working_dir: Option<&PathBuf>,
    ) -> io::Result<HashMap<String, Vec<String>>> {
        let memory_file_path = self.get_memory_file(category, is_global, working_dir);
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

    pub fn remove_specific_memory_internal(
        &self,
        category: &str,
        memory_content: &str,
        is_global: bool,
        working_dir: Option<&PathBuf>,
    ) -> io::Result<()> {
        let memory_file_path = self.get_memory_file(category, is_global, working_dir);
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

    pub fn clear_memory(
        &self,
        category: &str,
        is_global: bool,
        working_dir: Option<&PathBuf>,
    ) -> io::Result<()> {
        let memory_file_path = self.get_memory_file(category, is_global, working_dir);
        if memory_file_path.exists() {
            fs::remove_file(memory_file_path)?;
        }

        Ok(())
    }

    pub fn clear_all_global_or_local_memories(
        &self,
        is_global: bool,
        working_dir: Option<&PathBuf>,
    ) -> io::Result<()> {
        let base_dir = if is_global {
            self.global_memory_dir.clone()
        } else {
            let local_base = working_dir
                .cloned()
                .or_else(|| std::env::current_dir().ok())
                .unwrap_or_else(|| PathBuf::from("."));
            local_base.join(".goose").join("memory")
        };
        if base_dir.exists() {
            fs::remove_dir_all(&base_dir)?;
        }
        Ok(())
    }

    /// Stores a memory with optional tags in a specified category
    #[tool(
        name = "remember_memory",
        description = "Stores a memory with optional tags in a specified category"
    )]
    pub async fn remember_memory(
        &self,
        params: Parameters<RememberMemoryParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let params = params.0;
        let working_dir = extract_working_dir_from_meta(&context.meta);

        if params.data.is_empty() {
            return Err(ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                "Data must not be empty when remembering a memory".to_string(),
                None,
            ));
        }

        let tags: Vec<&str> = params.tags.iter().map(|s| s.as_str()).collect();
        self.remember(
            "context",
            &params.category,
            &params.data,
            &tags,
            params.is_global,
            working_dir.as_ref(),
        )
        .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Stored memory in category: {}",
            params.category
        ))]))
    }

    /// Retrieves all memories from a specified category
    #[tool(
        name = "retrieve_memories",
        description = "Retrieves all memories from a specified category"
    )]
    pub async fn retrieve_memories(
        &self,
        params: Parameters<RetrieveMemoriesParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let params = params.0;
        let working_dir = extract_working_dir_from_meta(&context.meta);

        let memories = if params.category == "*" {
            self.retrieve_all(params.is_global, working_dir.as_ref())
        } else {
            self.retrieve(&params.category, params.is_global, working_dir.as_ref())
        }
        .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Retrieved memories: {:?}",
            memories
        ))]))
    }

    /// Removes all memories within a specified category
    #[tool(
        name = "remove_memory_category",
        description = "Removes all memories within a specified category"
    )]
    pub async fn remove_memory_category(
        &self,
        params: Parameters<RemoveMemoryCategoryParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let params = params.0;
        let working_dir = extract_working_dir_from_meta(&context.meta);

        let message = if params.category == "*" {
            self.clear_all_global_or_local_memories(params.is_global, working_dir.as_ref())
                .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;
            format!(
                "Cleared all memory {} categories",
                if params.is_global { "global" } else { "local" }
            )
        } else {
            self.clear_memory(&params.category, params.is_global, working_dir.as_ref())
                .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;
            format!("Cleared memories in category: {}", params.category)
        };

        Ok(CallToolResult::success(vec![Content::text(message)]))
    }

    /// Removes a specific memory within a specified category
    #[tool(
        name = "remove_specific_memory",
        description = "Removes a specific memory within a specified category"
    )]
    pub async fn remove_specific_memory(
        &self,
        params: Parameters<RemoveSpecificMemoryParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let params = params.0;
        let working_dir = extract_working_dir_from_meta(&context.meta);

        self.remove_specific_memory_internal(
            &params.category,
            &params.memory_content,
            params.is_global,
            working_dir.as_ref(),
        )
        .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Removed specific memory from category: {}",
            params.category
        ))]))
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for MemoryServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            server_info: Implementation {
                name: "goose-memory".to_string(),
                version: env!("CARGO_PKG_VERSION").to_owned(),
                title: None,
                description: None,
                icons: None,
                website_url: None,
            },
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            instructions: Some(self.instructions.clone()),
            ..Default::default()
        }
    }
}

// Remove the old MemoryArgs struct since we're using the new parameter structs

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_lazy_directory_creation() {
        let temp_dir = tempdir().unwrap();
        let memory_base = temp_dir.path().join("test_memory");
        let working_dir = memory_base.join("working");

        let router = MemoryServer {
            tool_router: ToolRouter::new(),
            instructions: String::new(),
            global_memory_dir: memory_base.join("global"),
        };

        let local_memory_dir = working_dir.join(".goose").join("memory");

        assert!(!router.global_memory_dir.exists());
        assert!(!local_memory_dir.exists());

        router
            .remember(
                "test_context",
                "test_category",
                "test_data",
                &["tag1"],
                false,
                Some(&working_dir),
            )
            .unwrap();

        assert!(local_memory_dir.exists());
        assert!(!router.global_memory_dir.exists());

        router
            .remember(
                "test_context",
                "global_category",
                "global_data",
                &["global_tag"],
                true,
                None,
            )
            .unwrap();

        assert!(router.global_memory_dir.exists());
    }

    #[test]
    fn test_clear_nonexistent_directories() {
        let temp_dir = tempdir().unwrap();
        let memory_base = temp_dir.path().join("nonexistent_memory");
        let working_dir = memory_base.join("working");

        let router = MemoryServer {
            tool_router: ToolRouter::new(),
            instructions: String::new(),
            global_memory_dir: memory_base.join("global"),
        };

        assert!(router
            .clear_all_global_or_local_memories(false, Some(&working_dir))
            .is_ok());
        assert!(router
            .clear_all_global_or_local_memories(true, None)
            .is_ok());
    }

    #[test]
    fn test_remember_retrieve_clear_workflow() {
        let temp_dir = tempdir().unwrap();
        let memory_base = temp_dir.path().join("workflow_test");
        let working_dir = memory_base.join("working");

        let router = MemoryServer {
            tool_router: ToolRouter::new(),
            instructions: String::new(),
            global_memory_dir: memory_base.join("global"),
        };

        router
            .remember(
                "context",
                "test_category",
                "test_data_content",
                &["test_tag"],
                false,
                Some(&working_dir),
            )
            .unwrap();

        let memories = router
            .retrieve("test_category", false, Some(&working_dir))
            .unwrap();
        assert!(!memories.is_empty());

        let has_content = memories.values().any(|v| {
            v.iter()
                .any(|content| content.contains("test_data_content"))
        });
        assert!(has_content);

        router
            .clear_memory("test_category", false, Some(&working_dir))
            .unwrap();

        let memories_after_clear = router
            .retrieve("test_category", false, Some(&working_dir))
            .unwrap();
        assert!(memories_after_clear.is_empty());
    }

    #[test]
    fn test_directory_creation_on_write() {
        let temp_dir = tempdir().unwrap();
        let memory_base = temp_dir.path().join("write_test");
        let working_dir = memory_base.join("working");

        let router = MemoryServer {
            tool_router: ToolRouter::new(),
            instructions: String::new(),
            global_memory_dir: memory_base.join("global"),
        };

        let local_memory_dir = working_dir.join(".goose").join("memory");
        assert!(!local_memory_dir.exists());

        router
            .remember(
                "context",
                "category",
                "data",
                &[],
                false,
                Some(&working_dir),
            )
            .unwrap();

        assert!(local_memory_dir.exists());
        assert!(local_memory_dir.join("category.txt").exists());
    }

    #[test]
    fn test_remove_specific_memory() {
        let temp_dir = tempdir().unwrap();
        let memory_base = temp_dir.path().join("remove_test");
        let working_dir = memory_base.join("working");

        let router = MemoryServer {
            tool_router: ToolRouter::new(),
            instructions: String::new(),
            global_memory_dir: memory_base.join("global"),
        };

        router
            .remember(
                "context",
                "category",
                "keep_this",
                &[],
                false,
                Some(&working_dir),
            )
            .unwrap();
        router
            .remember(
                "context",
                "category",
                "remove_this",
                &[],
                false,
                Some(&working_dir),
            )
            .unwrap();

        let memories = router
            .retrieve("category", false, Some(&working_dir))
            .unwrap();
        assert_eq!(memories.len(), 1);

        router
            .remove_specific_memory_internal("category", "remove_this", false, Some(&working_dir))
            .unwrap();

        let memories_after = router
            .retrieve("category", false, Some(&working_dir))
            .unwrap();
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
