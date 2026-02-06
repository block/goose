use std::borrow::Cow;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use futures::FutureExt;
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use rmcp::model::{Content, ErrorCode, ErrorData, Tool};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::agents::tool_execution::ToolCallResult;
use crate::conversation::message::Message;
use crate::providers::{self, base::Provider};

pub const SUMMARIZE_TOOL_NAME: &str = "summarize";

#[derive(Debug, Deserialize)]
pub struct SummarizeParams {
    /// Files or directories to include. Directories are expanded recursively.
    pub paths: Vec<String>,

    /// What to focus on or ask about the content. This guides the summary.
    pub question: String,

    /// File extensions to include (e.g., ["rs", "py"]). If not specified, includes all files.
    pub extensions: Option<Vec<String>>,

    /// Provider/model settings override.
    pub settings: Option<SummarizeSettings>,
}

#[derive(Debug, Deserialize)]
pub struct SummarizeSettings {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    #[serde(flatten)]
    pub extra: Option<HashMap<String, Value>>,
}

pub fn create_summarize_tool() -> Tool {
    let schema = json!({
        "type": "object",
        "required": ["paths", "question"],
        "properties": {
            "paths": {
                "type": "array",
                "items": {"type": "string"},
                "description": "Files or directories to include. Directories are expanded recursively (respects .gitignore)."
            },
            "question": {
                "type": "string",
                "description": "What to focus on or ask about the content. This guides the summary."
            },
            "extensions": {
                "type": "array",
                "items": {"type": "string"},
                "description": "File extensions to include (e.g., [\"rs\", \"py\"]). If not specified, includes all files."
            },
            "settings": {
                "type": "object",
                "properties": {
                    "provider": {"type": "string", "description": "Override LLM provider"},
                    "model": {"type": "string", "description": "Override model"},
                    "temperature": {"type": "number", "description": "Override temperature"}
                },
                "additionalProperties": true,
                "description": "Override model/provider settings."
            }
        }
    });

    Tool::new(
        SUMMARIZE_TOOL_NAME,
        "Load files/directories deterministically and get an LLM summary in a single call. \
         More efficient than subagent when you know what to analyze. \
         Specify paths (files or dirs that will be recursively expanded, respecting .gitignore), \
         a question to focus the summary, and optionally filter by file extensions.",
        schema.as_object().unwrap().clone(),
    )
}

pub fn handle_summarize_tool(
    provider: Arc<dyn Provider>,
    session_id: String,
    params: Value,
    working_dir: PathBuf,
) -> ToolCallResult {
    let parsed_params: SummarizeParams = match serde_json::from_value(params) {
        Ok(p) => p,
        Err(e) => {
            return ToolCallResult::from(Err(ErrorData {
                code: ErrorCode::INVALID_PARAMS,
                message: Cow::from(format!("Invalid parameters: {}", e)),
                data: None,
            }));
        }
    };

    if parsed_params.paths.is_empty() {
        return ToolCallResult::from(Err(ErrorData {
            code: ErrorCode::INVALID_PARAMS,
            message: Cow::from("Must provide at least one path"),
            data: None,
        }));
    }

    ToolCallResult {
        notification_stream: None,
        result: Box::new(
            execute_summarize(provider, session_id, parsed_params, working_dir).boxed(),
        ),
    }
}

async fn execute_summarize(
    base_provider: Arc<dyn Provider>,
    session_id: String,
    params: SummarizeParams,
    working_dir: PathBuf,
) -> Result<rmcp::model::CallToolResult, ErrorData> {
    let provider = create_provider(base_provider, &params.settings).await?;

    let gitignore = build_gitignore(&working_dir);
    let files = collect_files(&params.paths, &working_dir, &params.extensions, &gitignore)
        .map_err(|e| ErrorData {
            code: ErrorCode::INTERNAL_ERROR,
            message: Cow::from(format!("Failed to collect files: {}", e)),
            data: None,
        })?;

    if files.is_empty() {
        return Err(ErrorData {
            code: ErrorCode::INVALID_PARAMS,
            message: Cow::from("No files found matching the specified paths and extensions."),
            data: None,
        });
    }

    let prompt = build_prompt(&files, &params.question, &working_dir);
    let total_lines: usize = files.iter().map(|f| f.lines).sum();
    let file_count = files.len();

    let system =
        "You are an assistant that analyzes content and provides clear, concise summaries \
                  focused on answering the user's specific question. \
                  Be specific and reference relevant parts of the content when helpful.";

    let user_message = Message::user().with_text(&prompt);

    let (response, _usage) = provider
        .complete(&session_id, system, &[user_message], &[])
        .await
        .map_err(|e| ErrorData {
            code: ErrorCode::INTERNAL_ERROR,
            message: Cow::from(format!("LLM call failed: {}", e)),
            data: None,
        })?;

    let response_text = response
        .content
        .iter()
        .filter_map(|c| {
            if let crate::conversation::message::MessageContent::Text(t) = c {
                Some(t.text.clone())
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    let metadata = format!(
        "\n\n---\n*Analyzed {} files ({} lines)*",
        file_count, total_lines
    );

    Ok(rmcp::model::CallToolResult {
        content: vec![Content::text(format!("{}{}", response_text, metadata))],
        structured_content: None,
        is_error: Some(false),
        meta: None,
    })
}

async fn create_provider(
    base_provider: Arc<dyn Provider>,
    settings: &Option<SummarizeSettings>,
) -> Result<Arc<dyn Provider>, ErrorData> {
    let Some(settings) = settings else {
        return Ok(base_provider);
    };

    let has_overrides = settings.provider.is_some()
        || settings.model.is_some()
        || settings.temperature.is_some()
        || settings.extra.as_ref().is_some_and(|e| !e.is_empty());

    if !has_overrides {
        return Ok(base_provider);
    }

    let provider_name = settings
        .provider
        .clone()
        .unwrap_or_else(|| base_provider.get_name().to_string());

    let mut model_config = base_provider.get_model_config();

    if let Some(model) = &settings.model {
        model_config.model_name = model.clone();
    }

    if let Some(temp) = settings.temperature {
        model_config = model_config.with_temperature(Some(temp));
    }

    if let Some(extra) = &settings.extra {
        let filtered: HashMap<String, Value> = extra
            .iter()
            .filter(|(k, _)| !matches!(k.as_str(), "provider" | "model" | "temperature"))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        if !filtered.is_empty() {
            model_config = model_config.with_request_params(Some(filtered));
        }
    }

    providers::create(&provider_name, model_config)
        .await
        .map_err(|e| ErrorData {
            code: ErrorCode::INTERNAL_ERROR,
            message: Cow::from(format!(
                "Failed to create provider '{}': {}",
                provider_name, e
            )),
            data: None,
        })
}

struct FileContent {
    path: PathBuf,
    content: String,
    lines: usize,
}

fn build_gitignore(working_dir: &Path) -> Gitignore {
    let mut builder = GitignoreBuilder::new(working_dir);

    // Add .gitignore if it exists
    let gitignore_path = working_dir.join(".gitignore");
    if gitignore_path.is_file() {
        let _ = builder.add(&gitignore_path);
    }

    // Always ignore .git directory
    let _ = builder.add_line(None, ".git/");

    builder.build().unwrap_or_else(|_| Gitignore::empty())
}

fn should_include_file(path: &Path, extensions: &Option<Vec<String>>) -> bool {
    match extensions {
        Some(exts) => {
            let ext = match path.extension().and_then(|e| e.to_str()) {
                Some(e) => e.to_lowercase(),
                None => return false,
            };
            exts.iter().any(|e| e.to_lowercase() == ext)
        }
        None => true,
    }
}

fn collect_files(
    paths: &[String],
    working_dir: &Path,
    extensions: &Option<Vec<String>>,
    gitignore: &Gitignore,
) -> Result<Vec<FileContent>, String> {
    let mut files = Vec::new();

    for path_str in paths {
        let path = if Path::new(path_str).is_absolute() {
            PathBuf::from(path_str)
        } else {
            working_dir.join(path_str)
        };

        if !path.exists() {
            return Err(format!("Path does not exist: {}", path.display()));
        }

        if path.is_dir() {
            collect_from_dir(&path, &mut files, extensions, gitignore)?;
        } else if path.is_file() && should_include_file(&path, extensions) {
            collect_file(&path, &mut files)?;
        }
    }

    Ok(files)
}

fn collect_from_dir(
    dir: &Path,
    files: &mut Vec<FileContent>,
    extensions: &Option<Vec<String>>,
    gitignore: &Gitignore,
) -> Result<(), String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("Failed to read directory: {}", e))?;

    let mut entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let path = entry.path();

        if gitignore.matched(&path, path.is_dir()).is_ignore() {
            continue;
        }

        if path.is_dir() {
            collect_from_dir(&path, files, extensions, gitignore)?;
        } else if path.is_file() && should_include_file(&path, extensions) {
            collect_file(&path, files)?;
        }
    }

    Ok(())
}

fn collect_file(path: &Path, files: &mut Vec<FileContent>) -> Result<(), String> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            tracing::debug!("Skipping file {}: {}", path.display(), e);
            return Ok(());
        }
    };

    let lines = content.lines().count();

    files.push(FileContent {
        path: path.to_owned(),
        content,
        lines,
    });

    Ok(())
}

fn build_prompt(files: &[FileContent], question: &str, working_dir: &Path) -> String {
    let total_lines: usize = files.iter().map(|f| f.lines).sum();
    let mut prompt =
        String::with_capacity(files.iter().map(|f| f.content.len()).sum::<usize>() + 1000);

    prompt.push_str(&format!("Answer this question: {}\n\n", question));
    prompt.push_str(&format!(
        "**Files** ({} files, {} total lines):\n\n",
        files.len(),
        total_lines
    ));

    for file in files {
        let display_path = file.path.strip_prefix(working_dir).unwrap_or(&file.path);
        let ext = file.path.extension().and_then(|e| e.to_str()).unwrap_or("");

        prompt.push_str(&format!(
            "### {} ({} lines)\n```{}\n{}\n```\n\n",
            display_path.display(),
            file.lines,
            ext,
            file.content
        ));
    }

    prompt
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_test_dir() -> TempDir {
        let dir = tempfile::tempdir().unwrap();

        fs::create_dir_all(dir.path().join("src")).unwrap();
        fs::write(
            dir.path().join("src/main.rs"),
            "fn main() {\n    println!(\"Hello\");\n}\n",
        )
        .unwrap();

        fs::write(
            dir.path().join("src/lib.rs"),
            "pub struct Foo;\n\nimpl Foo {\n    pub fn new() -> Self { Self }\n}\n",
        )
        .unwrap();

        fs::write(dir.path().join(".hidden"), "secret").unwrap();

        // Create .gitignore
        fs::write(dir.path().join(".gitignore"), "node_modules/\n*.log\n").unwrap();

        fs::create_dir_all(dir.path().join("node_modules")).unwrap();
        fs::write(
            dir.path().join("node_modules/pkg.js"),
            "module.exports = {}",
        )
        .unwrap();

        fs::write(dir.path().join("debug.log"), "some logs").unwrap();

        dir
    }

    #[test]
    fn test_collect_files_basic() {
        let dir = setup_test_dir();
        let gitignore = build_gitignore(dir.path());
        let files = collect_files(&["src".to_string()], dir.path(), &None, &gitignore).unwrap();

        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|f| f.path.ends_with("main.rs")));
        assert!(files.iter().any(|f| f.path.ends_with("lib.rs")));
    }

    #[test]
    fn test_collect_files_respects_gitignore() {
        let dir = setup_test_dir();
        let gitignore = build_gitignore(dir.path());
        let files = collect_files(&[".".to_string()], dir.path(), &None, &gitignore).unwrap();

        assert!(!files
            .iter()
            .any(|f| f.path.to_string_lossy().contains("node_modules")));
        assert!(!files
            .iter()
            .any(|f| f.path.to_string_lossy().contains(".log")));
    }

    #[test]
    fn test_collect_files_extension_filter() {
        let dir = setup_test_dir();
        fs::write(dir.path().join("src/script.py"), "print('hello')").unwrap();
        let gitignore = build_gitignore(dir.path());

        let files = collect_files(
            &["src".to_string()],
            dir.path(),
            &Some(vec!["py".to_string()]),
            &gitignore,
        )
        .unwrap();

        assert_eq!(files.len(), 1);
        assert!(files[0].path.ends_with("script.py"));
    }

    #[test]
    fn test_collect_files_no_extension_filter_includes_all() {
        let dir = setup_test_dir();
        fs::write(dir.path().join("src/README"), "readme content").unwrap();
        let gitignore = build_gitignore(dir.path());

        let files = collect_files(&["src".to_string()], dir.path(), &None, &gitignore).unwrap();

        // Should include files without extensions when no filter specified
        assert!(files.iter().any(|f| f.path.ends_with("README")));
    }

    #[test]
    fn test_collect_files_nonexistent_path() {
        let dir = setup_test_dir();
        let gitignore = build_gitignore(dir.path());
        let result = collect_files(&["nonexistent".to_string()], dir.path(), &None, &gitignore);

        assert!(result.is_err());
    }

    #[test]
    fn test_build_prompt() {
        let files = vec![FileContent {
            path: PathBuf::from("/project/src/main.rs"),
            content: "fn main() {}".to_string(),
            lines: 1,
        }];

        let prompt = build_prompt(&files, "How does main work?", Path::new("/project"));

        assert!(prompt.contains("How does main work?"));
        assert!(prompt.contains("src/main.rs"));
        assert!(prompt.contains("fn main() {}"));
        assert!(prompt.contains("```rs"));
    }

    #[test]
    fn test_tool_creation() {
        let tool = create_summarize_tool();
        assert_eq!(tool.name, SUMMARIZE_TOOL_NAME);
        assert!(tool.description.as_ref().unwrap().contains("deterministic"));
    }
}
