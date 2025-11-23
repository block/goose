use anyhow::anyhow;
use base64::Engine;
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use indoc::formatdoc;
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{
        CallToolResult, CancelledNotificationParam, Content, ErrorCode, ErrorData, Implementation,
        LoggingLevel, LoggingMessageNotificationParam, Role, ServerCapabilities, ServerInfo,
    },
    schemars::JsonSchema,
    service::{NotificationContext, RequestContext},
    tool, tool_handler, tool_router, RoleServer, ServerHandler,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    env::join_paths,
    ffi::OsString,
    io::Cursor,
    path::{Path, PathBuf},
    sync::Arc,
};

use tokio::{
    io::{AsyncBufReadExt, BufReader},
    sync::RwLock,
};
use tokio_stream::{wrappers::SplitStream, StreamExt as _};
use tokio_util::sync::CancellationToken;

use crate::developer::analyze::{types::AnalyzeParams, CodeAnalyzer};
use crate::developer::paths::get_shell_path_dirs;
use crate::developer::shell::{
    configure_shell_command, expand_path, is_absolute_path, kill_process_group, ShellConfig,
};

/// Parameters for the shell tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ShellParams {
    /// The command string to execute in the shell
    pub command: String,
}

/// Parameters for the image_processor tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ImageProcessorParams {
    /// Absolute path to the image file to process
    pub path: String,
}

/// Shell-focused MCP Server - all file operations via shell commands
#[derive(Clone)]
pub struct ShellServer {
    tool_router: ToolRouter<Self>,
    ignore_patterns: Gitignore,
    code_analyzer: CodeAnalyzer,
    #[cfg(test)]
    pub running_processes: Arc<RwLock<HashMap<String, CancellationToken>>>,
    #[cfg(not(test))]
    running_processes: Arc<RwLock<HashMap<String, CancellationToken>>>,
    bash_env_file: Option<PathBuf>,
    extend_path_with_shell: bool,
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for ShellServer {
    fn get_info(&self) -> ServerInfo {
        let cwd = std::env::current_dir().expect("should have a current working dir");
        let os = std::env::consts::OS;
        let in_container = Self::is_definitely_container();
        let shell_info = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());

        let instructions = formatdoc! {r#"
            The shell extension gives you capabilities to work with code through shell commands.
            You have access to tools to execute shell commands and read images.
            
            **All file operations must be done through shell commands.** You are an expert in 
            shell operations and can solve any problem using commands.

            ## Search

            **By filename:**
            ```bash
            rg --files | rg <filename>
            ```

            **By content:**
            ```bash
            rg 'pattern' -l                    # List files containing pattern
            rg 'pattern' -C 3                  # Show matches with 3 lines context
            rg 'class.*MyClass' --type rust    # Search in specific file types
            ```

            ## Read Files

            **Small files (< 100 lines):**
            ```bash
            cat file.txt
            ```

            **Large files - be surgical:**
            ```bash
            wc -l file.txt                     # Check size first!
            head -n 50 file.txt                # First 50 lines
            tail -n 50 file.txt                # Last 50 lines
            sed -n '10,20p' file.txt           # Lines 10-20
            nl file.txt | head -n 50           # With line numbers
            ```

            **Find and read specific sections:**
            ```bash
            # Find the line number, then read around it
            rg -n 'function_name' file.txt     # Shows line numbers
            sed -n '45,65p' file.txt           # Read lines 45-65
            ```

            ## Write/Edit Files

            **Replace text in files:**
            ```bash
            sed -i 's/old_text/new_text/' file.txt              # First occurrence per line
            sed -i 's/old_text/new_text/g' file.txt             # All occurrences (global)
            sed -i '10s/old_text/new_text/' file.txt            # Only line 10
            sed -i '/pattern/s/old_text/new_text/' file.txt     # Lines matching pattern
            ```

            **Insert lines:**
            ```bash
            sed -i '10i\new line content' file.txt              # Insert before line 10
            sed -i '10a\new line content' file.txt              # Insert after line 10
            sed -i '1i\new first line' file.txt                 # Insert at beginning
            sed -i '$a\new last line' file.txt                  # Append at end
            ```

            **Delete lines:**
            ```bash
            sed -i '10d' file.txt                               # Delete line 10
            sed -i '10,20d' file.txt                            # Delete lines 10-20
            sed -i '/pattern/d' file.txt                        # Delete lines matching pattern
            ```

            **Replace entire line:**
            ```bash
            sed -i '10c\replacement line' file.txt              # Replace line 10
            ```

            **Create or overwrite files:**
            ```bash
            # For small files, use heredoc
            cat > file.txt << 'EOF'
            Line 1
            Line 2
            Line 3
            EOF

            # For single lines
            echo "content" > file.txt

            # Append to file
            echo "more content" >> file.txt
            ```

            **Multi-line edits with heredoc:**
            ```bash
            # Replace a section (delete old lines, insert new)
            sed -i '10,15d' file.txt           # Delete lines 10-15 first
            sed -i '9r /dev/stdin' file.txt << 'EOF'    # Insert after line 9
            new line 1
            new line 2
            new line 3
            EOF
            ```

            ## Managing Context

            **You are responsible for managing your context window.** Be surgical:

            - **Check file size before reading:** `wc -l file.txt`
            - **Don't cat large files** - use `head`, `tail`, or `sed -n` for specific ranges
            - **Truncate command output:** `make 2>&1 | head -n 50` or redirect to file
            - **Use rg to locate, then sed to view:** Find line numbers first, then read specific ranges
            - **For large outputs, redirect to files:** `npm test > test.log 2>&1`

            ## Background Processes

            **Critical:** Long-running commands (servers, watchers) MUST be backgrounded!

            Your agent loop works like this:
            1. You invoke a tool
            2. The tool runs until fully complete
            3. You see the response
            4. You invoke the next tool

            You cannot run another tool until the first completes! So background long-running processes:

            ```bash
            # Start server in background
            npm start > server.log 2>&1 &
            echo $! > server.pid

            # Check if running
            ps -p $(cat server.pid)

            # View logs
            tail -f server.log   # This will block! Use head/tail instead
            tail -n 50 server.log

            # Stop when done
            kill $(cat server.pid)
            ```

            ## Shell Session State

            **IMPORTANT:** Each command runs in a fresh shell process. State does NOT persist:
            - `cd` commands do NOT persist between commands
            - `export` commands do NOT persist between commands
            - You must chain commands with `&&` when you need state:

            ```bash
            # WRONG - second command runs from original directory
            cd /project
            npm test

            # CORRECT - chain commands
            cd /project && npm test

            # CORRECT - multiple state changes
            cd /project && export DEBUG=1 && npm test
            ```




            ## Tips

            - **Always prefer ripgrep (`rg`)** over grep - it's faster and respects .gitignore
            - **Use `&&` to chain commands** - ensures each step succeeds
            - **Check exit codes:** `command && echo "success" || echo "failed"`
            - **For complex edits:** Sometimes easier to write a small script to a file, then execute it

            operating system: {os}
            current directory: {cwd}
            shell: {shell}
            {container_info}
        "#,
            os = os,
            cwd = cwd.to_string_lossy(),
            shell = shell_info,
            container_info = if in_container { "container: true" } else { "" },
        };

        ServerInfo {
            server_info: Implementation {
                name: "goose-shell".to_string(),
                version: env!("CARGO_PKG_VERSION").to_owned(),
                title: None,
                icons: None,
                website_url: None,
            },
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            instructions: Some(instructions),
            ..Default::default()
        }
    }

    /// Called when the client cancels a specific request
    #[allow(clippy::manual_async_fn)]
    fn on_cancelled(
        &self,
        notification: CancelledNotificationParam,
        _context: NotificationContext<RoleServer>,
    ) -> impl std::future::Future<Output = ()> + Send + '_ {
        async move {
            let request_id = notification.request_id.to_string();
            let processes = self.running_processes.read().await;

            if let Some(token) = processes.get(&request_id) {
                token.cancel();
                tracing::debug!("Found process for request {}, cancelling token", request_id);
            } else {
                tracing::warn!("No process found for request ID: {}", request_id);
            }
        }
    }
}

impl Default for ShellServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router(router = tool_router)]
impl ShellServer {
    pub fn new() -> Self {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let ignore_patterns = Self::build_ignore_patterns(&cwd);

        Self {
            tool_router: Self::tool_router(),
            ignore_patterns,
            code_analyzer: CodeAnalyzer::new(),
            running_processes: Arc::new(RwLock::new(HashMap::new())),
            extend_path_with_shell: false,
            bash_env_file: None,
        }
    }

    pub fn extend_path_with_shell(mut self, value: bool) -> Self {
        self.extend_path_with_shell = value;
        self
    }

    pub fn bash_env_file(mut self, value: Option<PathBuf>) -> Self {
        self.bash_env_file = value;
        self
    }

    /// Execute a command in the shell
    ///
    /// This will return the output and error concatenated into a single string.
    /// Each command runs in a fresh shell - chain with && for shared state.
    ///
    /// Avoid commands that produce large output - use head/tail/pipes.
    /// Background long-running commands with & to avoid blocking.
    #[tool(
        name = "shell",
        description = "Execute a command in the shell. Returns combined output. Each command runs independently - use && to chain commands that need shared state (cd, export). Avoid large outputs - use head/tail. Background long-running commands with &."
    )]
    pub async fn shell(
        &self,
        params: Parameters<ShellParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let params = params.0;
        let command = &params.command;
        let peer = context.peer;
        let request_id = context.id;

        // Validate the shell command
        self.validate_shell_command(command)?;

        let cancellation_token = CancellationToken::new();
        {
            let mut processes = self.running_processes.write().await;
            let request_id_str = request_id.to_string();
            processes.insert(request_id_str.clone(), cancellation_token.clone());
        }

        // Execute the wrapped command
        let output_result = self
            .execute_shell_command(command, &peer, cancellation_token.clone())
            .await;

        // Clean up the process from tracking
        {
            let mut processes = self.running_processes.write().await;
            let request_id_str = request_id.to_string();
            processes.remove(&request_id_str);
        }

        let output_str = output_result?;

        // Validate output size
        self.validate_shell_output_size(command, &output_str)?;

        // Process and format the output
        let (final_output, user_output) = self.process_shell_output(&output_str)?;

        Ok(CallToolResult::success(vec![
            Content::text(final_output).with_audience(vec![Role::Assistant]),
            Content::text(user_output)
                .with_audience(vec![Role::User])
                .with_priority(0.0),
        ]))
    }

    /// Analyze code structure and relationships.
    ///
    /// Automatically selects the appropriate analysis:
    /// - Files: Semantic analysis with call graphs
    /// - Directories: Structure overview with metrics
    /// - With focus parameter: Track symbol across files
    #[tool(
        name = "analyze",
        description = "Analyze code structure in 3 modes: 1) Directory overview - file tree with LOC/function/class counts to max_depth. 2) File details - functions, classes, imports. 3) Symbol focus - call graphs across directory to max_depth (requires directory path, case-sensitive). Typical flow: directory → files → symbols. Functions called >3x show •N."
    )]
    pub async fn analyze(
        &self,
        params: Parameters<AnalyzeParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let params = params.0;
        let path = self.resolve_path(&params.path)?;
        self.code_analyzer
            .analyze(params, path, &self.ignore_patterns)
    }

    /// Process an image file from disk.
    ///
    /// The image will be resized if needed, converted to PNG, and returned as base64.
    #[tool(
        name = "image_processor",
        description = "Process an image file from disk. Resizes if needed, converts to PNG, and returns as base64 data."
    )]
    pub async fn image_processor(
        &self,
        params: Parameters<ImageProcessorParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let params = params.0;
        let path_str = &params.path;

        let path = {
            let p = self.resolve_path(path_str)?;
            if cfg!(target_os = "macos") {
                self.normalize_mac_screenshot_path(&p)
            } else {
                p
            }
        };

        // Check if file is ignored
        if self.is_ignored(&path) {
            return Err(ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!(
                    "Access to '{}' is restricted by .gooseignore",
                    path.display()
                ),
                None,
            ));
        }

        // Check if file exists
        if !path.exists() {
            return Err(ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!("File '{}' does not exist", path.display()),
                None,
            ));
        }

        // Check file size (10MB limit)
        const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;
        let file_size = std::fs::metadata(&path)
            .map_err(|e| {
                ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Failed to get file metadata: {}", e),
                    None,
                )
            })?
            .len();

        if file_size > MAX_FILE_SIZE {
            return Err(ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!(
                    "File '{}' is too large ({:.2}MB). Maximum size is 10MB.",
                    path.display(),
                    file_size as f64 / (1024.0 * 1024.0)
                ),
                None,
            ));
        }

        // Open and decode the image
        let image = xcap::image::open(&path).map_err(|e| {
            ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!("Failed to open image file: {}", e),
                None,
            )
        })?;

        // Resize if necessary
        let mut processed_image = image;
        let max_width = 768;
        if processed_image.width() > max_width {
            let scale = max_width as f32 / processed_image.width() as f32;
            let new_height = (processed_image.height() as f32 * scale) as u32;
            processed_image = xcap::image::DynamicImage::ImageRgba8(xcap::image::imageops::resize(
                &processed_image,
                max_width,
                new_height,
                xcap::image::imageops::FilterType::Lanczos3,
            ));
        }

        // Convert to PNG and encode as base64
        let mut bytes: Vec<u8> = Vec::new();
        processed_image
            .write_to(&mut Cursor::new(&mut bytes), xcap::image::ImageFormat::Png)
            .map_err(|e| {
                ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Failed to write image buffer: {}", e),
                    None,
                )
            })?;

        let data = base64::prelude::BASE64_STANDARD.encode(bytes);

        Ok(CallToolResult::success(vec![
            Content::text(format!(
                "Successfully processed image from {}",
                path.display()
            ))
            .with_audience(vec![Role::Assistant]),
            Content::image(data, "image/png").with_priority(0.0),
        ]))
    }

    // Helper methods

    fn resolve_path(&self, path_str: &str) -> Result<PathBuf, ErrorData> {
        let cwd = std::env::current_dir().expect("should have a current working dir");
        let expanded = expand_path(path_str);
        let path = Path::new(&expanded);

        if is_absolute_path(&expanded) {
            Ok(path.to_path_buf())
        } else {
            Ok(cwd.join(path))
        }
    }

    fn build_ignore_patterns(cwd: &PathBuf) -> Gitignore {
        let mut builder = GitignoreBuilder::new(cwd);
        let local_ignore_path = cwd.join(".gooseignore");
        let mut has_ignore_file = false;

        if local_ignore_path.is_file() {
            let _ = builder.add(local_ignore_path);
            has_ignore_file = true;
        }

        if !has_ignore_file {
            let _ = builder.add_line(None, "**/.env");
            let _ = builder.add_line(None, "**/.env.*");
            let _ = builder.add_line(None, "**/secrets.*");
        }

        builder.build().expect("Failed to build ignore patterns")
    }

    fn is_ignored(&self, path: &Path) -> bool {
        self.ignore_patterns.matched(path, false).is_ignore()
    }

    fn is_definitely_container() -> bool {
        let Ok(content) = std::fs::read_to_string("/proc/1/cgroup") else {
            return false;
        };

        for line in content.lines() {
            if line.contains("/docker/")
                || line.contains("/docker-")
                || line.contains("/kubepods/")
                || line.contains("/libpod-")
                || line.contains("/lxc/")
                || line.contains("/containerd/")
            {
                return true;
            }
        }

        if content.trim() == "0::/" {
            return true;
        }

        false
    }

    fn normalize_mac_screenshot_path(&self, path: &Path) -> PathBuf {
        if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
            if let Some(captures) = regex::Regex::new(r"^Screenshot \d{4}-\d{2}-\d{2} at \d{1,2}\.\d{2}\.\d{2} (AM|PM|am|pm)(?: \(\d+\))?\.png$")
                .ok()
                .and_then(|re| re.captures(filename))
            {
                let meridian = captures.get(1).unwrap().as_str();
                let space_pos = filename.rfind(meridian)
                    .and_then(|pos| filename.get(..pos).map(|s| s.trim_end().len()))
                    .unwrap_or(0);

                if space_pos > 0 {
                    let parent = path.parent().unwrap_or(Path::new(""));
                    if let (Some(before), Some(after)) = (filename.get(..space_pos), filename.get(space_pos+1..)) {
                        let new_filename = format!(
                            "{}{}{}",
                            before,
                            '\u{202F}',
                            after
                        );
                        return parent.join(new_filename);
                    }
                }
            }
        }
        path.to_path_buf()
    }

    fn validate_shell_command(&self, command: &str) -> Result<(), ErrorData> {
        if command.trim().is_empty() {
            return Err(ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                "Shell command cannot be empty".to_string(),
                None,
            ));
        }

        let cmd_parts: Vec<&str> = command.split_whitespace().collect();

        for arg in &cmd_parts[1..] {
            if arg.starts_with('-') {
                continue;
            }

            let path = Path::new(arg);
            if !path.exists() {
                continue;
            }

            if self.is_ignored(path) {
                return Err(ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!(
                        "The command attempts to access '{}' which is restricted by .gooseignore",
                        arg
                    ),
                    None,
                ));
            }
        }

        Ok(())
    }

    async fn execute_shell_command(
        &self,
        command: &str,
        peer: &rmcp::service::Peer<RoleServer>,
        cancellation_token: CancellationToken,
    ) -> Result<String, ErrorData> {
        let mut shell_config = ShellConfig::default();
        let shell_name = std::path::Path::new(&shell_config.executable)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("bash");

        if let Some(ref env_file) = self.bash_env_file {
            if shell_name == "bash" {
                shell_config.envs.push((
                    OsString::from("BASH_ENV"),
                    env_file.clone().into_os_string(),
                ))
            }
        }

        let mut command_builder = configure_shell_command(&shell_config, command);

        if self.extend_path_with_shell {
            if let Err(e) = get_shell_path_dirs()
                .await
                .and_then(|dirs| join_paths(dirs).map_err(|e| anyhow!(e)))
                .map(|path| command_builder.env("PATH", path))
            {
                tracing::error!("Failed to extend PATH with shell directories: {}", e)
            }
        }

        let mut child = command_builder
            .spawn()
            .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;

        let pid = child.id();

        let output_task = self.stream_shell_output(
            child.stdout.take().unwrap(),
            child.stderr.take().unwrap(),
            peer.clone(),
        );

        tokio::select! {
            output_result = output_task => {
                let _exit_status = child.wait().await.map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;
                output_result
            }
            _ = cancellation_token.cancelled() => {
                tracing::info!("Cancellation token triggered! Attempting to kill process");

                match kill_process_group(&mut child, pid).await {
                    Ok(_) => {
                        tracing::debug!("Successfully killed shell process");
                    }
                    Err(e) => {
                        tracing::error!("Failed to kill shell process: {}", e);
                    }
                }

                Err(ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    "Shell command was cancelled by user".to_string(),
                    None,
                ))
            }
        }
    }

    async fn stream_shell_output(
        &self,
        stdout: tokio::process::ChildStdout,
        stderr: tokio::process::ChildStderr,
        peer: rmcp::service::Peer<RoleServer>,
    ) -> Result<String, ErrorData> {
        let stdout = BufReader::new(stdout);
        let stderr = BufReader::new(stderr);

        let output_task = tokio::spawn(async move {
            let mut combined_output = String::new();

            let stdout = SplitStream::new(stdout.split(b'\n')).map(|v| ("stdout", v));
            let stderr = SplitStream::new(stderr.split(b'\n')).map(|v| ("stderr", v));
            let mut merged = stdout.merge(stderr);

            while let Some((stream_type, line)) = merged.next().await {
                let mut line = line?;
                line.push(b'\n');
                let line_str = String::from_utf8_lossy(&line);

                combined_output.push_str(&line_str);

                let trimmed_line = line_str.trim();
                if !trimmed_line.is_empty() {
                    if let Err(e) = peer
                        .notify_logging_message(LoggingMessageNotificationParam {
                            level: LoggingLevel::Info,
                            data: serde_json::json!({
                                "type": "shell_output",
                                "stream": stream_type,
                                "output": trimmed_line
                            }),
                            logger: Some("shell_tool".to_string()),
                        })
                        .await
                    {
                        eprintln!("Failed to stream output line: {}", e);
                    }
                }
            }
            Ok::<_, std::io::Error>(combined_output)
        });

        match output_task.await {
            Ok(result) => {
                result.map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))
            }
            Err(e) => Err(ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                e.to_string(),
                None,
            )),
        }
    }

    fn validate_shell_output_size(&self, command: &str, output: &str) -> Result<(), ErrorData> {
        const MAX_CHAR_COUNT: usize = 400_000;
        let char_count = output.chars().count();

        if char_count > MAX_CHAR_COUNT {
            return Err(ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!(
                    "Shell output from command '{}' has too many characters ({}). Maximum is {}.",
                    command, char_count, MAX_CHAR_COUNT
                ),
                None,
            ));
        }

        Ok(())
    }

    fn process_shell_output(&self, output_str: &str) -> Result<(String, String), ErrorData> {
        let lines: Vec<&str> = output_str.lines().collect();
        let line_count = lines.len();

        let start = lines.len().saturating_sub(100);
        let last_100_lines_str = lines[start..].join("\n");

        let final_output = if line_count > 100 {
            let tmp_file = tempfile::NamedTempFile::new().map_err(|e| {
                ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Failed to create temporary file: {}", e),
                    None,
                )
            })?;

            std::fs::write(tmp_file.path(), output_str).map_err(|e| {
                ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Failed to write to temporary file: {}", e),
                    None,
                )
            })?;

            let (_, path) = tmp_file.keep().map_err(|e| {
                ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Failed to persist temporary file: {}", e),
                    None,
                )
            })?;

            format!(
                "private note: output was {} lines, showing most recent. remainder in {} - do not show tmp file to user. truncated output:\n{}",
                line_count,
                path.display(),
                last_100_lines_str
            )
        } else {
            output_str.to_string()
        };

        let user_output = if line_count > 100 {
            format!(
                "NOTE: Output was {} lines, showing only the last 100 lines.\n\n{}",
                line_count, last_100_lines_str
            )
        } else {
            output_str.to_string()
        };

        Ok((final_output, user_output))
    }
}
