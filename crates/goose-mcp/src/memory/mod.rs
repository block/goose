use cap_fs_ext::{DirExt, MetadataExt};
#[cfg(windows)]
use cap_std::fs::OpenOptionsExt;
use cap_std::{
    ambient_authority,
    fs::{Dir, OpenOptions},
};
use etcetera::{choose_app_strategy, AppStrategy};
use indoc::formatdoc;
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{
        CallToolResult, ContentBlock, ErrorCode, ErrorData, Implementation, InitializeResult,
        MetaObject, ServerCapabilities, ServerInfo,
    },
    schemars::JsonSchema,
    service::RequestContext,
    tool, tool_handler, tool_router, RoleServer, ServerHandler,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    ffi::{OsStr, OsString},
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

const WORKING_DIR_HEADER: &str = "agent-working-dir";
static NEXT_MEMORY_TEMP_FILE: AtomicU64 = AtomicU64::new(0);

enum MemoryFileOpen {
    Read,
    AppendOrCreate,
    CreateNew,
}

struct MemoryLocation {
    anchor: PathBuf,
    components: Vec<OsString>,
}

impl MemoryLocation {
    fn open(&self, create: bool) -> io::Result<Option<Dir>> {
        if create {
            fs::create_dir_all(&self.anchor)?;
        }

        let mut directory = match Dir::open_ambient_dir(&self.anchor, ambient_authority()) {
            Ok(directory) => directory,
            Err(error) if !create && error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };

        for component in &self.components {
            directory = match directory.open_dir_nofollow(component) {
                Ok(directory) => directory,
                Err(error) if create && error.kind() == io::ErrorKind::NotFound => {
                    match directory.create_dir(component) {
                        Ok(()) => {}
                        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                        Err(error) => return Err(error),
                    }
                    directory.open_dir_nofollow(component)?
                }
                Err(error) if !create && error.kind() == io::ErrorKind::NotFound => {
                    return Ok(None)
                }
                Err(error) => return Err(error),
            };
        }

        Ok(Some(directory))
    }
}

fn validate_memory_category(category: &str) -> io::Result<OsString> {
    if category.is_empty()
        || category == "*"
        || category == "."
        || category == ".."
        || category.contains('/')
        || category.contains('\\')
        || category.contains(':')
        || is_reserved_windows_category(category)
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "memory category must be a single filename component",
        ));
    }

    Ok(OsString::from(format!("{category}.txt")))
}

fn same_memory_file(left: &cap_std::fs::Metadata, right: &cap_std::fs::Metadata) -> bool {
    left.dev() == right.dev() && left.ino() == right.ino()
}

#[cfg(unix)]
fn restrict_memory_temp_file(file: &fs::File) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    file.set_permissions(fs::Permissions::from_mode(0o600))
}

#[cfg(not(unix))]
fn restrict_memory_temp_file(_file: &fs::File) -> io::Result<()> {
    Ok(())
}

fn preserve_memory_file_security(source: &fs::File, destination: &fs::File) -> io::Result<()> {
    destination.set_permissions(source.metadata()?.permissions())?;

    #[cfg(windows)]
    {
        use std::os::windows::io::AsRawHandle;
        use std::ptr;
        use winapi::shared::minwindef::HLOCAL;
        use winapi::shared::winerror::ERROR_SUCCESS;
        use winapi::um::accctrl::SE_FILE_OBJECT;
        use winapi::um::aclapi::{GetSecurityInfo, SetSecurityInfo};
        use winapi::um::securitybaseapi::GetSecurityDescriptorControl;
        use winapi::um::winbase::LocalFree;
        use winapi::um::winnt::{
            DACL_SECURITY_INFORMATION, PACL, PROTECTED_DACL_SECURITY_INFORMATION,
            PSECURITY_DESCRIPTOR, SE_DACL_PROTECTED, UNPROTECTED_DACL_SECURITY_INFORMATION,
        };

        let mut dacl: PACL = ptr::null_mut();
        let mut descriptor: PSECURITY_DESCRIPTOR = ptr::null_mut();
        let status = unsafe {
            GetSecurityInfo(
                source.as_raw_handle(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION,
                ptr::null_mut(),
                ptr::null_mut(),
                &mut dacl,
                ptr::null_mut(),
                &mut descriptor,
            )
        };
        if status != ERROR_SUCCESS {
            return Err(io::Error::from_raw_os_error(status as i32));
        }

        let mut control = 0;
        let mut revision = 0;
        if unsafe { GetSecurityDescriptorControl(descriptor, &mut control, &mut revision) } == 0 {
            let error = io::Error::last_os_error();
            unsafe {
                LocalFree(descriptor as HLOCAL);
            }
            return Err(error);
        }

        let protection = if control & SE_DACL_PROTECTED != 0 {
            PROTECTED_DACL_SECURITY_INFORMATION
        } else {
            UNPROTECTED_DACL_SECURITY_INFORMATION
        };
        let status = unsafe {
            SetSecurityInfo(
                destination.as_raw_handle(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION | protection,
                ptr::null_mut(),
                ptr::null_mut(),
                dacl,
                ptr::null_mut(),
            )
        };
        unsafe {
            LocalFree(descriptor as HLOCAL);
        }
        if status != ERROR_SUCCESS {
            return Err(io::Error::from_raw_os_error(status as i32));
        }
    }

    Ok(())
}

fn open_memory_file_at(
    directory: &Dir,
    name: &OsStr,
    intent: MemoryFileOpen,
) -> io::Result<fs::File> {
    let before_open = if !matches!(intent, MemoryFileOpen::CreateNew) {
        match directory.symlink_metadata(Path::new(name)) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "memory category must not be a symbolic link",
                ));
            }
            Ok(metadata) => Some(metadata),
            Err(error)
                if matches!(intent, MemoryFileOpen::AppendOrCreate)
                    && error.kind() == io::ErrorKind::NotFound =>
            {
                None
            }
            Err(error) => return Err(error),
        }
    } else {
        None
    };

    let mut options = OpenOptions::new();
    match intent {
        MemoryFileOpen::Read => {
            options.read(true);
        }
        MemoryFileOpen::AppendOrCreate => {
            options.append(true).create(true);
        }
        MemoryFileOpen::CreateNew => {
            options.write(true).create_new(true);
            #[cfg(windows)]
            options.access_mode({
                use winapi::um::winnt::{GENERIC_READ, GENERIC_WRITE, READ_CONTROL, WRITE_DAC};

                GENERIC_READ | GENERIC_WRITE | READ_CONTROL | WRITE_DAC
            });
        }
    }
    // Capability-relative resolution confines any swap that lands during open, while the
    // identity checks reject observing a different in-tree file through the race window.
    let file = directory.open_with(Path::new(name), &options)?;
    let opened_metadata = file.metadata()?;
    let current_metadata = directory.symlink_metadata(Path::new(name))?;
    if current_metadata.file_type().is_symlink()
        || !same_memory_file(&opened_metadata, &current_metadata)
        || before_open
            .as_ref()
            .is_some_and(|metadata| !same_memory_file(metadata, &opened_metadata))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "memory category changed while it was being opened",
        ));
    }
    if !opened_metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "memory category must be a regular file",
        ));
    }
    Ok(file.into_std())
}

fn is_reserved_windows_category(category: &str) -> bool {
    let basename = category
        .split('.')
        .next()
        .unwrap_or(category)
        .trim_end_matches([' ', '.']);
    let uppercase = basename.to_ascii_uppercase();

    matches!(uppercase.as_str(), "CON" | "PRN" | "AUX" | "NUL" | "CLOCK$")
        || ["COM", "LPT"].iter().any(|prefix| {
            uppercase.strip_prefix(prefix).is_some_and(|suffix| {
                matches!(
                    suffix,
                    "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                )
            })
        })
}

fn extract_working_dir_from_meta(meta: &MetaObject) -> Option<PathBuf> {
    meta.0
        .get(WORKING_DIR_HEADER)
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
}

fn memory_error(error: io::Error) -> ErrorData {
    let code = if error.kind() == io::ErrorKind::InvalidInput {
        ErrorCode::INVALID_PARAMS
    } else {
        ErrorCode::INTERNAL_ERROR
    };
    ErrorData::new(code, error.to_string(), None)
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
             This extension stores and retrieves categorized information with tagging support.

             Storage:
             - Local: .goose/memory/ (project-specific)
             - Global: ~/.config/goose/memory/ (user-wide)

             Save proactively when users share preferences, project configurations, workflow patterns,
             or recurring commands. Always confirm with the user before saving. Suggest relevant
             categories and tags, and clarify storage scope (local vs global).

             Use category "*" with retrieve_memories or remove_memory_category to access all entries.
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

    fn memory_location(
        &self,
        is_global: bool,
        working_dir: Option<&PathBuf>,
    ) -> io::Result<MemoryLocation> {
        if is_global {
            let component = self
                .global_memory_dir
                .file_name()
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "global memory path must name a directory",
                    )
                })?
                .to_os_string();
            let parent = self
                .global_memory_dir
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty())
                .map(Path::to_path_buf)
                .map(Ok)
                .unwrap_or_else(std::env::current_dir)?;
            Ok(MemoryLocation {
                anchor: parent,
                components: vec![component],
            })
        } else {
            let anchor = working_dir
                .cloned()
                .map(Ok)
                .unwrap_or_else(std::env::current_dir)?;
            Ok(MemoryLocation {
                anchor,
                components: vec![OsString::from(".goose"), OsString::from("memory")],
            })
        }
    }

    fn open_memory_directory(
        &self,
        is_global: bool,
        working_dir: Option<&PathBuf>,
        create: bool,
    ) -> io::Result<Option<Dir>> {
        self.memory_location(is_global, working_dir)?.open(create)
    }

    fn retrieve_from_directory(
        &self,
        directory: &Dir,
        category: &str,
    ) -> io::Result<HashMap<String, Vec<String>>> {
        let file_name = validate_memory_category(category)?;
        let mut file = match open_memory_file_at(directory, &file_name, MemoryFileOpen::Read) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(HashMap::new()),
            Err(error) => return Err(error),
        };
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

    fn replace_memory_file(
        &self,
        directory: &Dir,
        file_name: &OsStr,
        content: &[u8],
        source: &fs::File,
    ) -> io::Result<()> {
        let process_id = std::process::id();
        let (temp_name, mut temp_file) = loop {
            let sequence = NEXT_MEMORY_TEMP_FILE.fetch_add(1, Ordering::Relaxed);
            let temp_name = OsString::from(format!(".goose-memory-{process_id}-{sequence}.tmp"));
            match open_memory_file_at(directory, &temp_name, MemoryFileOpen::CreateNew) {
                Ok(file) => break (temp_name, file),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        };

        let result = (|| {
            restrict_memory_temp_file(&temp_file)?;
            temp_file.write_all(content)?;
            preserve_memory_file_security(source, &temp_file)?;
            temp_file.sync_all()?;
            drop(temp_file);
            directory.rename(Path::new(&temp_name), directory, Path::new(file_name))
        })();
        if result.is_err() {
            let _ = directory.remove_file_or_symlink(Path::new(&temp_name));
        }
        result
    }

    pub fn retrieve_all(
        &self,
        is_global: bool,
        working_dir: Option<&PathBuf>,
    ) -> io::Result<HashMap<String, Vec<String>>> {
        let Some(directory) = self.open_memory_directory(is_global, working_dir, false)? else {
            return Ok(HashMap::new());
        };
        let mut memories = HashMap::new();
        for entry in directory.entries()? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                let file_name = entry.file_name();
                let Some(category) = file_name
                    .to_str()
                    .and_then(|name| name.strip_suffix(".txt"))
                else {
                    continue;
                };
                if validate_memory_category(category).is_err() {
                    continue;
                }
                let category_memories = self.retrieve_from_directory(&directory, category)?;
                memories.insert(
                    category.to_string(),
                    category_memories.into_values().flatten().collect(),
                );
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
        let file_name = validate_memory_category(category)?;
        let directory = self
            .open_memory_directory(is_global, working_dir, true)?
            .expect("creating a memory directory returns an open directory");
        let mut file = open_memory_file_at(&directory, &file_name, MemoryFileOpen::AppendOrCreate)?;
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
        validate_memory_category(category)?;
        let Some(directory) = self.open_memory_directory(is_global, working_dir, false)? else {
            return Ok(HashMap::new());
        };
        self.retrieve_from_directory(&directory, category)
    }

    pub fn remove_specific_memory_internal(
        &self,
        category: &str,
        memory_content: &str,
        is_global: bool,
        working_dir: Option<&PathBuf>,
    ) -> io::Result<()> {
        let file_name = validate_memory_category(category)?;
        let Some(directory) = self.open_memory_directory(is_global, working_dir, false)? else {
            return Ok(());
        };
        let mut file = match open_memory_file_at(&directory, &file_name, MemoryFileOpen::Read) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error),
        };
        let mut content = String::new();
        file.read_to_string(&mut content)?;

        let memories: Vec<&str> = content.split("\n\n").collect();
        let new_content: Vec<String> = memories
            .into_iter()
            .filter(|entry| !entry.contains(memory_content))
            .map(|s| s.to_string())
            .collect();

        self.replace_memory_file(
            &directory,
            &file_name,
            new_content.join("\n\n").as_bytes(),
            &file,
        )
    }

    pub fn clear_memory(
        &self,
        category: &str,
        is_global: bool,
        working_dir: Option<&PathBuf>,
    ) -> io::Result<()> {
        let file_name = validate_memory_category(category)?;
        let Some(directory) = self.open_memory_directory(is_global, working_dir, false)? else {
            return Ok(());
        };
        match directory.remove_file_or_symlink(Path::new(&file_name)) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
        }
    }

    pub fn clear_all_global_or_local_memories(
        &self,
        is_global: bool,
        working_dir: Option<&PathBuf>,
    ) -> io::Result<()> {
        let Some(directory) = self.open_memory_directory(is_global, working_dir, false)? else {
            return Ok(());
        };
        directory.remove_open_dir_all()
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
        .map_err(memory_error)?;

        Ok(CallToolResult::success(vec![ContentBlock::text(format!(
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
        .map_err(memory_error)?;

        Ok(CallToolResult::success(vec![ContentBlock::text(format!(
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
                .map_err(memory_error)?;
            format!(
                "Cleared all memory {} categories",
                if params.is_global { "global" } else { "local" }
            )
        } else {
            self.clear_memory(&params.category, params.is_global, working_dir.as_ref())
                .map_err(memory_error)?;
            format!("Cleared memories in category: {}", params.category)
        };

        Ok(CallToolResult::success(vec![ContentBlock::text(message)]))
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
        .map_err(memory_error)?;

        Ok(CallToolResult::success(vec![ContentBlock::text(format!(
            "Removed specific memory from category: {}",
            params.category
        ))]))
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for MemoryServer {
    fn get_info(&self) -> ServerInfo {
        InitializeResult::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(
                "goose-memory",
                env!("CARGO_PKG_VERSION"),
            ))
            .with_instructions(self.instructions.clone())
    }
}

// Remove the old MemoryArgs struct since we're using the new parameter structs

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[cfg(unix)]
    fn assert_category_operations_reject_link(
        router: &MemoryServer,
        working_dir: &PathBuf,
        outside_file: &Path,
    ) {
        assert!(router
            .retrieve("category", false, Some(working_dir))
            .is_err());
        assert!(router
            .remember(
                "context",
                "category",
                "malicious append",
                &[],
                false,
                Some(working_dir),
            )
            .is_err());
        assert!(
            router
                .remove_specific_memory_internal(
                    "category",
                    "outside secret",
                    false,
                    Some(working_dir),
                )
                .is_err()
        );
        assert_eq!(fs::read_to_string(outside_file).unwrap(), "outside secret");
    }

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

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            fs::set_permissions(
                working_dir.join(".goose/memory/category.txt"),
                fs::Permissions::from_mode(0o600),
            )
            .unwrap();
        }

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

        let memory_dir = working_dir.join(".goose/memory");
        assert_eq!(fs::read_dir(memory_dir).unwrap().count(), 1);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let mode = fs::metadata(working_dir.join(".goose/memory/category.txt"))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600);
        }
    }

    #[cfg(windows)]
    fn set_owner_only_protected_dacl(path: &Path) {
        use std::os::windows::{fs::OpenOptionsExt, io::AsRawHandle};
        use std::ptr;
        use winapi::shared::minwindef::HLOCAL;
        use winapi::shared::sddl::{
            ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
        };
        use winapi::shared::winerror::ERROR_SUCCESS;
        use winapi::um::accctrl::SE_FILE_OBJECT;
        use winapi::um::aclapi::SetSecurityInfo;
        use winapi::um::securitybaseapi::GetSecurityDescriptorDacl;
        use winapi::um::winbase::LocalFree;
        use winapi::um::winnt::{
            DACL_SECURITY_INFORMATION, PACL, PROTECTED_DACL_SECURITY_INFORMATION,
            PSECURITY_DESCRIPTOR, READ_CONTROL, WRITE_DAC,
        };

        let sddl: Vec<u16> = "D:P(A;;FA;;;OW)\0".encode_utf16().collect();
        let mut descriptor: PSECURITY_DESCRIPTOR = ptr::null_mut();
        assert_ne!(
            unsafe {
                ConvertStringSecurityDescriptorToSecurityDescriptorW(
                    sddl.as_ptr(),
                    SDDL_REVISION_1 as u32,
                    &mut descriptor,
                    ptr::null_mut(),
                )
            },
            0
        );

        let mut dacl: PACL = ptr::null_mut();
        let mut dacl_present = 0;
        let mut dacl_defaulted = 0;
        assert_ne!(
            unsafe {
                GetSecurityDescriptorDacl(
                    descriptor,
                    &mut dacl_present,
                    &mut dacl,
                    &mut dacl_defaulted,
                )
            },
            0
        );
        assert_ne!(dacl_present, 0);

        let file = fs::OpenOptions::new()
            .access_mode(READ_CONTROL | WRITE_DAC)
            .open(path)
            .unwrap();
        let status = unsafe {
            SetSecurityInfo(
                file.as_raw_handle(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
                ptr::null_mut(),
                ptr::null_mut(),
                dacl,
                ptr::null_mut(),
            )
        };
        unsafe {
            LocalFree(descriptor as HLOCAL);
        }
        assert_eq!(status, ERROR_SUCCESS);
    }

    #[cfg(windows)]
    fn assert_owner_only_protected_dacl(path: &Path) {
        use std::ffi::c_void;
        use std::os::windows::io::AsRawHandle;
        use std::ptr;
        use winapi::shared::minwindef::HLOCAL;
        use winapi::shared::winerror::ERROR_SUCCESS;
        use winapi::um::accctrl::SE_FILE_OBJECT;
        use winapi::um::aclapi::GetSecurityInfo;
        use winapi::um::securitybaseapi::{
            CreateWellKnownSid, EqualSid, GetAce, GetSecurityDescriptorControl,
        };
        use winapi::um::winbase::LocalFree;
        use winapi::um::winnt::{
            WinCreatorOwnerRightsSid, ACCESS_ALLOWED_ACE, ACCESS_ALLOWED_ACE_TYPE,
            DACL_SECURITY_INFORMATION, FILE_ALL_ACCESS, PACL, PSECURITY_DESCRIPTOR, PSID,
            SECURITY_MAX_SID_SIZE, SE_DACL_PROTECTED,
        };

        let file = fs::File::open(path).unwrap();
        let mut dacl: PACL = ptr::null_mut();
        let mut descriptor: PSECURITY_DESCRIPTOR = ptr::null_mut();
        let status = unsafe {
            GetSecurityInfo(
                file.as_raw_handle(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION,
                ptr::null_mut(),
                ptr::null_mut(),
                &mut dacl,
                ptr::null_mut(),
                &mut descriptor,
            )
        };
        assert_eq!(status, ERROR_SUCCESS);

        let mut control = 0;
        let mut revision = 0;
        assert_ne!(
            unsafe { GetSecurityDescriptorControl(descriptor, &mut control, &mut revision) },
            0
        );
        assert_ne!(control & SE_DACL_PROTECTED, 0);
        assert!(!dacl.is_null());
        assert_eq!(unsafe { (*dacl).AceCount }, 1);

        let mut ace: *mut c_void = ptr::null_mut();
        assert_ne!(unsafe { GetAce(dacl, 0, &mut ace) }, 0);
        let allowed = ace.cast::<ACCESS_ALLOWED_ACE>();
        assert_eq!(
            unsafe { (*allowed).Header.AceType },
            ACCESS_ALLOWED_ACE_TYPE
        );
        assert_eq!(
            unsafe { (*allowed).Mask } & FILE_ALL_ACCESS,
            FILE_ALL_ACCESS
        );

        let mut expected_sid = [0u8; SECURITY_MAX_SID_SIZE];
        let mut expected_sid_size = expected_sid.len() as u32;
        assert_ne!(
            unsafe {
                CreateWellKnownSid(
                    WinCreatorOwnerRightsSid,
                    ptr::null_mut(),
                    expected_sid.as_mut_ptr().cast(),
                    &mut expected_sid_size,
                )
            },
            0
        );
        let actual_sid = unsafe { &mut (*allowed).SidStart as *mut u32 as PSID };
        assert_ne!(
            unsafe { EqualSid(actual_sid, expected_sid.as_mut_ptr().cast()) },
            0
        );
        unsafe {
            LocalFree(descriptor as HLOCAL);
        }
    }

    #[cfg(windows)]
    #[test]
    fn test_remove_specific_memory_preserves_windows_dacl() {
        let temp_dir = tempdir().unwrap();
        let working_dir = temp_dir.path().join("working");
        let router = MemoryServer {
            tool_router: ToolRouter::new(),
            instructions: String::new(),
            global_memory_dir: temp_dir.path().join("global"),
        };

        router
            .remember(
                "context",
                "category",
                "keep",
                &[],
                false,
                Some(&working_dir),
            )
            .unwrap();
        router
            .remember(
                "context",
                "category",
                "remove",
                &[],
                false,
                Some(&working_dir),
            )
            .unwrap();

        let category = working_dir.join(".goose/memory/category.txt");
        set_owner_only_protected_dacl(&category);

        router
            .remove_specific_memory_internal("category", "remove", false, Some(&working_dir))
            .unwrap();

        assert_owner_only_protected_dacl(&category);
    }

    #[cfg(unix)]
    #[test]
    fn test_category_operations_do_not_follow_file_symlinks() {
        use std::os::unix::fs::symlink;

        let temp_dir = tempdir().unwrap();
        let working_dir = temp_dir.path().join("working");
        let memory_dir = working_dir.join(".goose/memory");
        let outside_file = temp_dir.path().join("outside.txt");
        fs::create_dir_all(&memory_dir).unwrap();
        fs::write(&outside_file, "outside secret").unwrap();
        symlink(&outside_file, memory_dir.join("category.txt")).unwrap();

        let router = MemoryServer {
            tool_router: ToolRouter::new(),
            instructions: String::new(),
            global_memory_dir: temp_dir.path().join("global"),
        };

        assert_category_operations_reject_link(&router, &working_dir, &outside_file);
        assert!(fs::symlink_metadata(memory_dir.join("category.txt"))
            .unwrap()
            .file_type()
            .is_symlink());
    }

    #[cfg(unix)]
    #[test]
    fn test_category_operations_do_not_follow_memory_directory_symlink() {
        use std::os::unix::fs::symlink;

        let temp_dir = tempdir().unwrap();
        let working_dir = temp_dir.path().join("working");
        let goose_dir = working_dir.join(".goose");
        let outside_dir = temp_dir.path().join("outside-memory");
        let outside_file = outside_dir.join("category.txt");
        fs::create_dir_all(&goose_dir).unwrap();
        fs::create_dir(&outside_dir).unwrap();
        fs::write(&outside_file, "outside secret").unwrap();
        symlink(&outside_dir, goose_dir.join("memory")).unwrap();

        let router = MemoryServer {
            tool_router: ToolRouter::new(),
            instructions: String::new(),
            global_memory_dir: temp_dir.path().join("global"),
        };

        assert_category_operations_reject_link(&router, &working_dir, &outside_file);
    }

    #[cfg(unix)]
    #[test]
    fn test_category_operations_do_not_follow_goose_directory_symlink() {
        use std::os::unix::fs::symlink;

        let temp_dir = tempdir().unwrap();
        let working_dir = temp_dir.path().join("working");
        let outside_dir = temp_dir.path().join("outside-goose");
        let outside_memory_dir = outside_dir.join("memory");
        let outside_file = outside_memory_dir.join("category.txt");
        fs::create_dir(&working_dir).unwrap();
        fs::create_dir_all(&outside_memory_dir).unwrap();
        fs::write(&outside_file, "outside secret").unwrap();
        symlink(&outside_dir, working_dir.join(".goose")).unwrap();

        let router = MemoryServer {
            tool_router: ToolRouter::new(),
            instructions: String::new(),
            global_memory_dir: temp_dir.path().join("global"),
        };

        assert_category_operations_reject_link(&router, &working_dir, &outside_file);
    }

    #[test]
    fn test_memory_operations_reject_escape_capable_categories() {
        let temp_dir = tempdir().unwrap();
        let working_dir = temp_dir.path().join("working");
        let outside_file = temp_dir.path().join("outside.txt");
        fs::write(&outside_file, "secret").unwrap();

        let router = MemoryServer {
            tool_router: ToolRouter::new(),
            instructions: String::new(),
            global_memory_dir: temp_dir.path().join("global"),
        };

        for category in [
            "",
            "*",
            ".",
            "..",
            "../../../outside",
            "/tmp/outside",
            r"..\..\outside",
            "C:outside",
            r"C:\outside",
            "NUL",
            "con",
            "AUX.log",
            "COM1",
            "lpt9",
        ] {
            assert_eq!(
                router
                    .remember(
                        "context",
                        category,
                        "malicious",
                        &[],
                        false,
                        Some(&working_dir)
                    )
                    .unwrap_err()
                    .kind(),
                io::ErrorKind::InvalidInput
            );
            assert_eq!(
                router
                    .retrieve(category, false, Some(&working_dir))
                    .unwrap_err()
                    .kind(),
                io::ErrorKind::InvalidInput
            );
            assert_eq!(
                router
                    .remove_specific_memory_internal(category, "secret", false, Some(&working_dir),)
                    .unwrap_err()
                    .kind(),
                io::ErrorKind::InvalidInput
            );
            assert_eq!(
                router
                    .clear_memory(category, false, Some(&working_dir))
                    .unwrap_err()
                    .kind(),
                io::ErrorKind::InvalidInput
            );
        }

        assert_eq!(fs::read_to_string(outside_file).unwrap(), "secret");
        assert!(!working_dir.join(".goose").exists());
    }

    #[test]
    fn test_memory_category_allows_safe_filename_characters() {
        let temp_dir = tempdir().unwrap();
        let working_dir = temp_dir.path().join("working");
        let router = MemoryServer {
            tool_router: ToolRouter::new(),
            instructions: String::new(),
            global_memory_dir: temp_dir.path().join("global"),
        };

        router
            .remember(
                "context",
                "project notes_2026",
                "safe",
                &[],
                false,
                Some(&working_dir),
            )
            .unwrap();

        assert!(working_dir
            .join(".goose/memory/project notes_2026.txt")
            .is_file());
    }

    #[cfg(unix)]
    #[test]
    fn test_retrieve_all_skips_invalid_legacy_categories() {
        let temp_dir = tempdir().unwrap();
        let working_dir = temp_dir.path().join("working");
        let memory_dir = working_dir.join(".goose/memory");
        fs::create_dir_all(&memory_dir).unwrap();
        fs::write(memory_dir.join("valid.txt"), "kept").unwrap();
        fs::write(memory_dir.join("work:api.txt"), "legacy").unwrap();
        fs::write(memory_dir.join(r"work\api.txt"), "legacy").unwrap();

        let router = MemoryServer {
            tool_router: ToolRouter::new(),
            instructions: String::new(),
            global_memory_dir: temp_dir.path().join("global"),
        };

        let memories = router.retrieve_all(false, Some(&working_dir)).unwrap();

        assert_eq!(memories.len(), 1);
        assert!(memories["valid"].iter().any(|entry| entry == "kept"));
    }

    #[test]
    fn test_memory_error_preserves_invalid_parameter_distinction() {
        let invalid = memory_error(io::Error::new(io::ErrorKind::InvalidInput, "bad category"));
        assert_eq!(invalid.code, ErrorCode::INVALID_PARAMS);

        let filesystem = memory_error(io::Error::new(io::ErrorKind::PermissionDenied, "denied"));
        assert_eq!(filesystem.code, ErrorCode::INTERNAL_ERROR);
    }
}
