use crate::errors::{AgentError, AgentResult};
use crate::systems::System;
use anyhow::Result as AnyhowResult;
use async_trait::async_trait;
use indoc::formatdoc;
use mcp_core::{Content, Resource, Tool, ToolCall};
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::io::{self, Read, Write};
use std::path::PathBuf;

#[derive(Debug, Default)]
pub struct MemoryManager {
    global_memory_dir: PathBuf,
    local_memory_dir: PathBuf,
}

impl MemoryManager {
    pub fn new() -> io::Result<Self> {
        // Check for .goose/memory in current directory
        let local_memory_dir = std::env::current_dir()
            .unwrap()
            .join(".goose")
            .join("memory");

        // Check for .config/goose/memory in user's home directory
        let global_memory_dir = dirs::home_dir()
            .map(|home| home.join(".config/goose/memory"))
            .unwrap_or_else(|| PathBuf::from(".config/goose/memory"));

        fs::create_dir_all(&global_memory_dir)?;
        fs::create_dir_all(&local_memory_dir)?;

        Ok(MemoryManager {
            global_memory_dir,
            local_memory_dir,
        })
    }

    fn get_memory_file(&self, category: &str, is_global: bool) -> PathBuf {
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
}

// Tool implementations

pub fn memory_tools() -> Vec<Tool> {
    vec![
        Tool::new(
            "remember_memory",
            "Stores a memory with optional tags in a specified category",
            json!({
                "type": "object",
                "properties": {
                    "category": {"type": "string"},
                    "data": {"type": "string"},
                    "tags": {"type": "array", "items": {"type": "string"}},
                    "is_global": {"type": "boolean"}
                },
                "required": ["category", "data", "is_global"]
            }),
        ),
        Tool::new(
            "retrieve_memories",
            "Retrieves all memories from a specified category",
            json!({
                "type": "object",
                "properties": {
                    "category": {"type": "string"},
                    "is_global": {"type": "boolean"}
                },
                "required": ["category", "is_global"]
            }),
        ),
        Tool::new(
            "remove_memory_category",
            "Removes all memories within a specified category",
            json!({
                "type": "object",
                "properties": {
                    "category": {"type": "string"},
                    "is_global": {"type": "boolean"}
                },
                "required": ["category", "is_global"]
            }),
        ),
        Tool::new(
            "remove_specific_memory",
            "Removes a specific memory within a specified category",
            json!({
                "type": "object",
                "properties": {
                    "category": {"type": "string"},
                    "memory_content": {"type": "string"},
                    "is_global": {"type": "boolean"}
                },
                "required": ["category", "memory_content", "is_global"]
            }),
        ),
    ]
}

pub fn execute_tool_call(tool_call: ToolCall) -> Result<String, io::Error> {
    match tool_call.name.as_str() {
        "remember_memory" => {
            let category = tool_call.arguments["category"].as_str().unwrap();
            let data = tool_call.arguments["data"].as_str().unwrap();
            let tags: Vec<&str> = tool_call.arguments["tags"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap())
                .collect();
            let is_global = tool_call.arguments["is_global"].as_bool().unwrap();
            MemoryManager::new()?.remember("context", category, data, &tags, is_global)?;
            Ok(format!("Stored memory in category: {}", category))
        }
        "retrieve_memories" => {
            let category = tool_call.arguments["category"].as_str().unwrap();
            let is_global = tool_call.arguments["is_global"].as_bool().unwrap();
            let memories = MemoryManager::new()?.retrieve(category, is_global)?;
            Ok(format!("Retrieved memories: {:?}", memories))
        }
        "remove_memory_category" => {
            let category = tool_call.arguments["category"].as_str().unwrap();
            let is_global = tool_call.arguments["is_global"].as_bool().unwrap();
            MemoryManager::new()?.clear_memory(category, is_global)?;
            Ok(format!("Cleared memories in category: {}", category))
        }
        "remove_specific_memory" => {
            let category = tool_call.arguments["category"].as_str().unwrap();
            let memory_content = tool_call.arguments["memory_content"].as_str().unwrap();
            let is_global = tool_call.arguments["is_global"].as_bool().unwrap();
            MemoryManager::new()?.remove_specific_memory(category, memory_content, is_global)?;
            Ok(format!(
                "Removed specific memory from category: {}",
                category
            ))
        }
        _ => Err(io::Error::new(io::ErrorKind::InvalidInput, "Unknown tool")),
    }
}

// MemorySystem implementation

pub struct MemorySystem {
    memory_tools: Vec<Tool>,
    active_memories: HashMap<String, Vec<String>>,
    instructions: String,
}

impl Default for MemorySystem {
    fn default() -> Self {
        Self::new()
    }
}

impl MemorySystem {
    pub fn new() -> Self {
        let memory_manager = MemoryManager::new().expect("Failed to create MemoryManager");

        // Load memories from global and local contexts
        let mut combined_memories = HashMap::new();
        for is_global in [true, false] {
            if let Ok(memories) = memory_manager.retrieve_all(is_global) {
                for (category, memory_list) in memories {
                    combined_memories
                        .entry(category.clone())
                        .or_insert_with(Vec::new)
                        .extend(memory_list);
                }
            }
        }

        let instructions = formatdoc! {r#"
             Memory Management System for Goose

             This system allows storage and retrieval of categorized information with tagging support. It's designed to help
             manage important information across sessions in a systematic and organized manner.

             Capabilities:
             1. Store information in categories with optional tags for context-based retrieval.
             2. Search memories by content or specific tags to find relevant information.
             3. List all available memory categories for easy navigation.
             4. Remove entire categories of memories when they are no longer needed.

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

             Retrieving Memories:
             To access stored information, utilize the memory retrieval protocols:

             - **Search by Category**:
               - Provides all memories within the specified context.
               - Use: `retrieve_memories(category="development", is_global=False)`

             - **Filter by Tags**:
               - Enables targeted retrieval based on specific tags.
               - Use: Provide tag filters to refine search.

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
        Self {
            memory_tools: memory_tools(),
            active_memories: combined_memories, // store loaded memories here
            instructions,
        }
    }
}

#[async_trait]
impl System for MemorySystem {
    fn name(&self) -> &str {
        "MemorySystem"
    }

    fn description(&self) -> &str {
        "System to manage and utilize memories for different contexts."
    }

    fn instructions(&self) -> &str {
        self.instructions.as_str()
    }

    fn tools(&self) -> &[Tool] {
        &self.memory_tools
    }

    async fn status(&self) -> AnyhowResult<Vec<Resource>> {
        // Convert active memories to resources
        let resources: Vec<Resource> = self
            .active_memories
            .iter()
            .filter_map(|(category, memories)| {
                Resource::with_uri(
                    format!("str:///{}.txt", memories.join(" ")),
                    format!("{}.txt", category),
                    0.0,
                    Some("text".to_string()),
                )
                .ok()
            })
            .collect();
        Ok(resources)
    }

    async fn call(&self, tool_call: ToolCall) -> AgentResult<Vec<Content>> {
        match execute_tool_call(tool_call) {
            Ok(result) => Ok(vec![Content::text(result)]),
            Err(err) => Err(AgentError::ExecutionError(err.to_string())),
        }
    }

    async fn read_resource(&self, uri: &str) -> AgentResult<String> {
        let memories = uri.split("/").next().unwrap();
        Ok(memories.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::{tempdir, TempDir};

    fn setup_memory_manager(temp_dir: &TempDir) -> MemoryManager {
        let global_memory_dir = temp_dir.path().join("global");
        let local_memory_dir = temp_dir.path().join("local");

        fs::create_dir_all(&global_memory_dir).unwrap();
        fs::create_dir_all(&local_memory_dir).unwrap();

        MemoryManager {
            global_memory_dir,
            local_memory_dir,
        }
    }

    #[test]
    fn test_remember_and_retrieve_memory() {
        let temp_dir = tempdir().unwrap();
        let memory_manager = setup_memory_manager(&temp_dir);

        // Remember a memory
        let category = "test_category";
        let data = "This is a test memory.";
        let tags = &["tag1", "tag2"];

        // Test remembering in local scope
        memory_manager
            .remember("", category, data, tags, false)
            .unwrap();

        // Retrieve the memory
        let retrieved = memory_manager.retrieve(category, false).unwrap();
        let retrieved_memories = retrieved.get(&tags.join(" ")).unwrap();

        assert_eq!(retrieved_memories.len(), 1);
        assert_eq!(retrieved_memories[0], data);

        // Remember another memory in global scope
        let global_data = "This is a global memory.";
        memory_manager
            .remember("", category, global_data, &[], true)
            .unwrap();

        let global_retrieved = memory_manager.retrieve(category, true).unwrap();
        let global_memories = global_retrieved.get("untagged").unwrap();

        assert_eq!(global_memories.len(), 1);
        assert_eq!(global_memories[0], global_data);
    }

    #[test]
    fn test_remove_specific_memory() {
        let temp_dir = tempdir().unwrap();
        let memory_manager = setup_memory_manager(&temp_dir);

        let category = "test_remove";
        let data1 = "Memory to keep.";
        let data2 = "Memory to remove.";

        // Remember multiple memories
        memory_manager
            .remember("", category, data1, &[], false)
            .unwrap();
        memory_manager
            .remember("", category, data2, &[], false)
            .unwrap();

        // Remove a specific memory
        memory_manager
            .remove_specific_memory(category, data2, false)
            .unwrap();

        // Retrieve and verify
        let retrieved = memory_manager.retrieve(category, false).unwrap();
        let untagged_memories = retrieved.get("untagged").unwrap();

        assert_eq!(untagged_memories.len(), 1);
        assert_eq!(untagged_memories[0], data1);
        assert!(!untagged_memories.contains(&data2.to_string()));
    }
}
