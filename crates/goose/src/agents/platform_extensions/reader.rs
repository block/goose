use crate::agents::extension::PlatformExtensionContext;
use crate::agents::mcp_client::{Error, McpClientTrait};
use anyhow::Result;
use async_trait::async_trait;
use goose_mcp::developer::text_editor;
use rmcp::model::{
    CallToolResult, Content, Implementation, InitializeResult, JsonObject, ListToolsResult,
    ProtocolVersion, ServerCapabilities, Tool, ToolAnnotations, ToolsCapability,
};
use schemars::JsonSchema;
use serde::Deserialize;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use tokio_util::sync::CancellationToken;

pub static EXTENSION_NAME: &str = "reader";
const MAX_FILE_BYTES: u64 = 256 * 1024;
const MAX_DIR_ENTRIES: usize = 2000;

#[derive(Debug, Deserialize, JsonSchema)]
struct ReadParams {
    path: String,
    #[serde(default = "default_depth")]
    max_depth: u32,
    #[serde(default)]
    view_range: Option<Vec<i64>>,
}
fn default_depth() -> u32 {
    3
}

fn resolve(raw: &str, wd: &Path) -> PathBuf {
    if Path::new(raw).is_absolute() {
        PathBuf::from(raw)
    } else {
        wd.join(raw)
    }
}

fn is_binary(path: &Path) -> bool {
    let Ok(mut f) = fs::File::open(path) else {
        return false;
    };
    let mut buf = [0u8; 8192];
    let Ok(n) = f.read(&mut buf) else {
        return false;
    };
    buf[..n].contains(&0)
}

fn read_file(
    path: &Path,
    view_range: Option<Vec<i64>>,
) -> std::result::Result<Vec<Content>, String> {
    if is_binary(path) {
        return Ok(vec![Content::text(format!(
            "Skipped binary file: {}",
            path.display()
        ))]);
    }
    let mut buf = Vec::new();
    let bytes_read = fs::File::open(path)
        .map_err(|e| format!("Cannot open '{}': {e}", path.display()))?
        .take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut buf)
        .map_err(|e| format!("Read error: {e}"))?;
    let truncated = bytes_read as u64 > MAX_FILE_BYTES;
    if truncated {
        buf.truncate(MAX_FILE_BYTES as usize);
    }
    let raw = String::from_utf8_lossy(&buf);
    let lines: Vec<&str> = raw.lines().collect();
    let total = lines.len();
    let te_range = match view_range.as_deref() {
        Some([a, b]) => {
            if *a < 0 {
                return Err("view_range start must be non-negative".into());
            }
            if *b < -1 {
                return Err("view_range end must be -1 or non-negative".into());
            }
            Some((*a as usize, *b))
        }
        Some(r) => {
            return Err(format!(
                "view_range must have exactly 2 elements, got {}",
                r.len()
            ))
        }
        None => None,
    };
    let (s, e) = text_editor::calculate_view_range(te_range, total).map_err(|e| e.message)?;
    let mut content = text_editor::format_file_content(path, &lines, s, e, te_range);
    if truncated {
        content.push_str("\n[truncated at 256KB]");
    }
    Ok(vec![Content::text(content)])
}

fn read_directory(path: &Path, max_depth: u32) -> Vec<Content> {
    let mut out = format!("{}\n", path.display());
    let mut stats = TreeStats {
        files: 0,
        dirs: 0,
        total: 0,
        truncated: false,
    };
    build_tree(path, 0, max_depth as usize, &mut out, &mut stats);
    if stats.truncated {
        out.push_str(&format!("\n(truncated at {MAX_DIR_ENTRIES} entries)\n"));
    }
    out.push_str(&format!(
        "\n{} files, {} directories\n",
        stats.files, stats.dirs
    ));
    vec![Content::text(out)]
}

struct TreeStats {
    files: usize,
    dirs: usize,
    total: usize,
    truncated: bool,
}

fn build_tree(dir: &Path, depth: usize, max_depth: usize, out: &mut String, stats: &mut TreeStats) {
    if stats.truncated || depth > max_depth {
        return;
    }
    let Ok(rd) = fs::read_dir(dir) else { return };
    let mut entries: Vec<fs::DirEntry> = rd
        .filter_map(|e| e.ok())
        .take(MAX_DIR_ENTRIES + 1 - stats.total.min(MAX_DIR_ENTRIES))
        .collect();
    let had_more = entries.len() > MAX_DIR_ENTRIES.saturating_sub(stats.total);
    entries.sort_by_key(|e| e.file_name());
    let n = entries.len();
    for (i, entry) in entries.iter().enumerate() {
        if stats.truncated {
            return;
        }
        let Ok(meta) = entry.metadata() else { continue };
        let name = entry.file_name().to_string_lossy().to_string();
        let pfx = format!(
            "{}{}",
            "│   ".repeat(depth),
            if i == n - 1 {
                "└── "
            } else {
                "├── "
            }
        );
        if meta.is_dir() {
            stats.dirs += 1;
            stats.total += 1;
            if stats.total > MAX_DIR_ENTRIES {
                stats.truncated = true;
                return;
            }
            out.push_str(&format!("{pfx}{name}/\n"));
            if depth < max_depth {
                build_tree(&entry.path(), depth + 1, max_depth, out, stats);
            }
        } else if meta.is_file() {
            stats.files += 1;
            stats.total += 1;
            if stats.total > MAX_DIR_ENTRIES {
                stats.truncated = true;
                return;
            }
            let sz = meta.len();
            let sz_s = match sz {
                0..=1023 => format!("{sz}B"),
                1024..=1_048_575 => format!("{:.1}KB", sz as f64 / 1024.0),
                _ => format!("{:.1}MB", sz as f64 / 1_048_576.0),
            };
            out.push_str(&format!("{pfx}{name:<40} {sz_s}\n"));
        }
    }
    if had_more && !stats.truncated {
        stats.truncated = true;
    }
}

pub struct ReaderClient {
    info: InitializeResult,
}

impl ReaderClient {
    pub fn new(_ctx: PlatformExtensionContext) -> Result<Self> {
        Ok(Self {
            info: InitializeResult {
                protocol_version: ProtocolVersion::V_2025_03_26,
                capabilities: ServerCapabilities {
                    tools: Some(ToolsCapability {
                        list_changed: Some(false),
                    }),
                    ..Default::default()
                },
                server_info: Implementation {
                    name: EXTENSION_NAME.into(),
                    title: Some("Reader".into()),
                    version: "1.0.0".into(),
                    ..Default::default()
                },
                instructions: Some(
                    "Read-only filesystem access. Use `read` to view files or list directories."
                        .into(),
                ),
            },
        })
    }

    fn get_tools() -> Vec<Tool> {
        let schema = schemars::schema_for!(ReadParams);
        let v = serde_json::to_value(schema).expect("schema");
        vec![Tool::new(
            "read",
            "Read a file or list a directory. Relative paths resolve against working directory.",
            v.as_object().unwrap().clone(),
        )
        .annotate(ToolAnnotations {
            title: Some("Read files and directories".into()),
            read_only_hint: Some(true),
            idempotent_hint: Some(true),
            ..Default::default()
        })]
    }

    fn handle_read(&self, p: ReadParams, wd: &Path) -> std::result::Result<Vec<Content>, String> {
        let path = resolve(&p.path, wd);
        let depth = p.max_depth.min(10);
        if path.is_dir() {
            Ok(read_directory(&path, depth))
        } else if path.is_file() {
            read_file(&path, p.view_range)
        } else {
            Err(format!("Not a file or directory: {}", path.display()))
        }
    }
}

#[async_trait]
impl McpClientTrait for ReaderClient {
    async fn list_tools(
        &self,
        _: &str,
        _: Option<String>,
        _: CancellationToken,
    ) -> std::result::Result<ListToolsResult, Error> {
        Ok(ListToolsResult {
            tools: Self::get_tools(),
            next_cursor: None,
            meta: None,
        })
    }

    async fn call_tool(
        &self,
        _: &str,
        name: &str,
        arguments: Option<JsonObject>,
        working_dir: Option<&str>,
        _: CancellationToken,
    ) -> std::result::Result<CallToolResult, Error> {
        if name != "read" {
            return Ok(CallToolResult::error(vec![Content::text(format!(
                "Unknown tool: {name}"
            ))]));
        }
        let wd = match working_dir.map(PathBuf::from) {
            Some(p) => p,
            None => match std::env::current_dir() {
                Ok(p) => p,
                Err(e) => {
                    return Ok(CallToolResult::error(vec![Content::text(format!(
                        "No working directory: {e}"
                    ))]));
                }
            },
        };
        let params: ReadParams = match arguments
            .map(|a| serde_json::from_value(serde_json::Value::Object(a)))
            .transpose()
        {
            Ok(Some(p)) => p,
            Ok(None) => ReadParams {
                path: ".".into(),
                max_depth: 3,
                view_range: None,
            },
            Err(e) => {
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "Invalid parameters: {e}"
                ))]))
            }
        };
        match self.handle_read(params, &wd) {
            Ok(c) => Ok(CallToolResult::success(c)),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Error: {e}"
            ))])),
        }
    }

    fn get_info(&self) -> Option<&InitializeResult> {
        Some(&self.info)
    }
}
