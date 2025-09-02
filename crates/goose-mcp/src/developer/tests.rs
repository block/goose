use super::*;
use core::panic;
use serde_json::json;
use serial_test::serial;
use std::fs::read_to_string;
use tempfile::TempDir;
use tokio::sync::OnceCell;

static DEV_ROUTER: OnceCell<DeveloperRouter> = OnceCell::const_new();

async fn get_router() -> &'static DeveloperRouter {
    DEV_ROUTER
        .get_or_init(|| async { DeveloperRouter::new() })
        .await
}

fn dummy_sender() -> mpsc::Sender<JsonRpcMessage> {
    mpsc::channel(1).0
}

#[tokio::test]
#[serial]
async fn test_shell_missing_parameters() {
    let temp_dir = tempfile::tempdir().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    let router = get_router().await;
    let result = router.call_tool("shell", json!({}), dummy_sender()).await;

    assert!(result.is_err());
    let err = result.err().unwrap();
    assert_eq!(err.code, ErrorCode::INVALID_PARAMS);

    temp_dir.close().unwrap();
}

#[tokio::test]
#[serial]
#[cfg(windows)]
async fn test_windows_specific_commands() {
    let router = get_router().await;

    // Test PowerShell command
    let result = router
        .call_tool(
            "shell",
            json!({
                "command": "Get-ChildItem"
            }),
            dummy_sender(),
        )
        .await;
    assert!(result.is_ok());

    // Test Windows path handling
    let result = router.resolve_path("C:\\Windows\\System32");
    assert!(result.is_ok());

    // Test UNC path handling
    let result = router.resolve_path("\\\\server\\share");
    assert!(result.is_ok());
}

#[tokio::test]
#[serial]
async fn test_text_editor_size_limits() {
    // Create temp directory first so it stays in scope for the whole test
    let temp_dir = tempfile::tempdir().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    // Get router after setting current directory
    let router = get_router().await;

    // Test file size limit
    {
        let large_file_path = temp_dir.path().join("large.txt");
        let large_file_str = large_file_path.to_str().unwrap();

        // Create a file larger than 2MB
        let content = "x".repeat(3 * 1024 * 1024); // 3MB
        std::fs::write(&large_file_path, content).unwrap();

        let result = router
            .call_tool(
                "text_editor",
                json!({
                    "command": "view",
                    "path": large_file_str
                }),
                dummy_sender(),
            )
            .await;

        assert!(result.is_err());
        let err = result.err().unwrap();
        assert_eq!(err.code, ErrorCode::INTERNAL_ERROR);
        assert!(err.to_string().contains("too large"));
    }

    // Test character count limit
    {
        let many_chars_path = temp_dir.path().join("many_chars.txt");
        let many_chars_str = many_chars_path.to_str().unwrap();

        // This is above MAX_FILE_SIZE
        let content = "x".repeat(500_000);
        std::fs::write(&many_chars_path, content).unwrap();

        let result = router
            .call_tool(
                "text_editor",
                json!({
                    "command": "view",
                    "path": many_chars_str
                }),
                dummy_sender(),
            )
            .await;

        assert!(result.is_err());
        let err = result.err().unwrap();
        assert_eq!(err.code, ErrorCode::INTERNAL_ERROR);
        assert!(err.to_string().contains("is too large"));
    }

    // Let temp_dir drop naturally at end of scope
}

#[tokio::test]
#[serial]
async fn test_text_editor_write_and_view_file() {
    let router = get_router().await;

    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    let file_path_str = file_path.to_str().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    // Create a new file
    router
        .call_tool(
            "text_editor",
            json!({
                "command": "write",
                "path": file_path_str,
                "file_text": "Hello, world!"
            }),
            dummy_sender(),
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
            dummy_sender(),
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
    assert!(text.text.contains("Hello, world!"));

    temp_dir.close().unwrap();
}

#[tokio::test]
#[serial]
async fn test_text_editor_str_replace() {
    let router = get_router().await;

    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    let file_path_str = file_path.to_str().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    // Create a new file
    router
        .call_tool(
            "text_editor",
            json!({
                "command": "write",
                "path": file_path_str,
                "file_text": "Hello, world!"
            }),
            dummy_sender(),
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
            dummy_sender(),
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

    assert!(text
        .text
        .contains("has been edited, and the section now reads"));

    // View the file to verify the change
    let view_result = router
        .call_tool(
            "text_editor",
            json!({
                "command": "view",
                "path": file_path_str
            }),
            dummy_sender(),
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

    // Check that the file has been modified and contains some form of "Rust"
    // The Editor API might transform the content differently than simple string replacement
    assert!(
        text.text.contains("Rust") || text.text.contains("Hello, Rust!"),
        "Expected content to contain 'Rust', but got: {}",
        text.text
    );

    temp_dir.close().unwrap();
}

#[tokio::test]
#[serial]
async fn test_text_editor_str_replace_multiple_matches_error() {
    let router = get_router().await;

    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    let file_path_str = file_path.to_str().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    // Create a file with multiple occurrences of the same string
    router
        .call_tool(
            "text_editor",
            json!({
                "command": "write",
                "path": file_path_str,
                "file_text": "Hello world! This world is a wonderful world."
            }),
            dummy_sender(),
        )
        .await
        .unwrap();

    // Try to replace "world" without replace_all (should fail)
    let result = router
        .call_tool(
            "text_editor",
            json!({
                "command": "str_replace",
                "path": file_path_str,
                "old_str": "world",
                "new_str": "universe"
            }),
            dummy_sender(),
        )
        .await;

    assert!(result.is_err());
    let err = result.err().unwrap();
    assert_eq!(err.code, ErrorCode::INVALID_PARAMS);
    assert!(err.to_string().contains("appears 3 times in the file"));
    assert!(err.to_string().contains("replace_all to true"));

    temp_dir.close().unwrap();
}

#[tokio::test]
#[serial]
async fn test_text_editor_str_replace_with_replace_all_true() {
    let router = get_router().await;

    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    let file_path_str = file_path.to_str().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    // Create a file with multiple occurrences of the same string
    router
        .call_tool(
            "text_editor",
            json!({
                "command": "write",
                "path": file_path_str,
                "file_text": "Hello world! This world is a wonderful world."
            }),
            dummy_sender(),
        )
        .await
        .unwrap();

    // Replace all occurrences of "world" with replace_all=true
    let replace_result = router
        .call_tool(
            "text_editor",
            json!({
                "command": "str_replace",
                "path": file_path_str,
                "old_str": "world",
                "new_str": "universe",
                "replace_all": true
            }),
            dummy_sender(),
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

    assert!(text.text.contains("has been edited"));

    // View the file to verify all occurrences were replaced
    let view_result = router
        .call_tool(
            "text_editor",
            json!({
                "command": "view",
                "path": file_path_str
            }),
            dummy_sender(),
        )
        .await
        .unwrap();

    let view_text = view_result
        .iter()
        .find(|c| {
            c.audience()
                .is_some_and(|roles| roles.contains(&Role::User))
        })
        .unwrap()
        .as_text()
        .unwrap();

    // All "world" should be replaced with "universe"
    assert!(view_text
        .text
        .contains("Hello universe! This universe is a wonderful universe."));
    assert!(!view_text.text.contains("world"));

    temp_dir.close().unwrap();
}

#[tokio::test]
#[serial]
async fn test_text_editor_str_replace_with_replace_all_false_explicit() {
    let router = get_router().await;

    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    let file_path_str = file_path.to_str().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    // Create a file with multiple occurrences of the same string
    router
        .call_tool(
            "text_editor",
            json!({
                "command": "write",
                "path": file_path_str,
                "file_text": "Hello world! This world is a wonderful world."
            }),
            dummy_sender(),
        )
        .await
        .unwrap();

    // Try to replace with explicit replace_all=false (should fail)
    let result = router
        .call_tool(
            "text_editor",
            json!({
                "command": "str_replace",
                "path": file_path_str,
                "old_str": "world",
                "new_str": "universe",
                "replace_all": false
            }),
            dummy_sender(),
        )
        .await;

    assert!(result.is_err());
    let err = result.err().unwrap();
    assert_eq!(err.code, ErrorCode::INVALID_PARAMS);
    assert!(err.to_string().contains("appears 3 times in the file"));
    assert!(err.to_string().contains("replace_all to true"));

    temp_dir.close().unwrap();
}

#[tokio::test]
#[serial]
async fn test_text_editor_str_replace_no_matches_with_replace_all() {
    let router = get_router().await;

    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    let file_path_str = file_path.to_str().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    // Create a file without the target string
    router
        .call_tool(
            "text_editor",
            json!({
                "command": "write",
                "path": file_path_str,
                "file_text": "Hello universe! This is great."
            }),
            dummy_sender(),
        )
        .await
        .unwrap();

    // Try to replace a string that doesn't exist, even with replace_all=true
    let result = router
        .call_tool(
            "text_editor",
            json!({
                "command": "str_replace",
                "path": file_path_str,
                "old_str": "world",
                "new_str": "planet",
                "replace_all": true
            }),
            dummy_sender(),
        )
        .await;

    assert!(result.is_err());
    let err = result.err().unwrap();
    assert_eq!(err.code, ErrorCode::INVALID_PARAMS);
    assert!(err.to_string().contains("does not appear in the file"));

    temp_dir.close().unwrap();
}

#[tokio::test]
#[serial]
async fn test_text_editor_undo_edit() {
    let router = get_router().await;

    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    let file_path_str = file_path.to_str().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    // Create a new file
    router
        .call_tool(
            "text_editor",
            json!({
                "command": "write",
                "path": file_path_str,
                "file_text": "First line"
            }),
            dummy_sender(),
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
            dummy_sender(),
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
            dummy_sender(),
        )
        .await
        .unwrap();

    let text = undo_result.first().unwrap().as_text().unwrap();
    assert!(text.text.contains("Undid the last edit"));

    // View the file to verify the undo
    let view_result = router
        .call_tool(
            "text_editor",
            json!({
                "command": "view",
                "path": file_path_str
            }),
            dummy_sender(),
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
    assert!(text.text.contains("First line"));

    temp_dir.close().unwrap();
}

// Test GooseIgnore pattern matching
#[tokio::test]
#[serial]
async fn test_goose_ignore_basic_patterns() {
    let temp_dir = tempfile::tempdir().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    // Create a DeveloperRouter with custom ignore patterns
    let mut builder = GitignoreBuilder::new(temp_dir.path());
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

#[tokio::test]
#[serial]
async fn test_text_editor_respects_ignore_patterns() {
    let temp_dir = tempfile::tempdir().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    // Create a DeveloperRouter with custom ignore patterns
    let mut builder = GitignoreBuilder::new(temp_dir.path());
    builder.add_line(None, "secret.txt").unwrap();
    let ignore_patterns = builder.build().unwrap();

    let router = DeveloperRouter {
        tools: DeveloperRouter::new().tools, // Reuse default tools
        prompts: Arc::new(HashMap::new()),
        instructions: String::new(),
        file_history: Arc::new(Mutex::new(HashMap::new())),
        ignore_patterns: Arc::new(ignore_patterns),
        editor_model: None,
    };

    // Try to write to an ignored file
    let result = router
        .call_tool(
            "text_editor",
            json!({
                "command": "write",
                "path": temp_dir.path().join("secret.txt").to_str().unwrap(),
                "file_text": "test content"
            }),
            dummy_sender(),
        )
        .await;

    assert!(
        result.is_err(),
        "Should not be able to write to ignored file"
    );
    assert_eq!(result.unwrap_err().code, ErrorCode::INTERNAL_ERROR);

    // Try to write to a non-ignored file
    let result = router
        .call_tool(
            "text_editor",
            json!({
                "command": "write",
                "path": temp_dir.path().join("allowed.txt").to_str().unwrap(),
                "file_text": "test content"
            }),
            dummy_sender(),
        )
        .await;

    assert!(
        result.is_ok(),
        "Should be able to write to non-ignored file"
    );

    temp_dir.close().unwrap();
}

#[tokio::test]
#[serial]
async fn test_bash_respects_ignore_patterns() {
    let temp_dir = tempfile::tempdir().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    // Create a DeveloperRouter with custom ignore patterns
    let mut builder = GitignoreBuilder::new(temp_dir.path());
    builder.add_line(None, "secret.txt").unwrap();
    let ignore_patterns = builder.build().unwrap();

    let router = DeveloperRouter {
        tools: DeveloperRouter::new().tools, // Reuse default tools
        prompts: Arc::new(HashMap::new()),
        instructions: String::new(),
        file_history: Arc::new(Mutex::new(HashMap::new())),
        ignore_patterns: Arc::new(ignore_patterns),
        editor_model: None,
    };

    // Create an ignored file
    let secret_file_path = temp_dir.path().join("secret.txt");
    std::fs::write(&secret_file_path, "secret content").unwrap();

    // Try to cat the ignored file
    let result = router
        .call_tool(
            "shell",
            json!({
                "command": format!("cat {}", secret_file_path.to_str().unwrap())
            }),
            dummy_sender(),
        )
        .await;

    assert!(result.is_err(), "Should not be able to cat ignored file");
    assert_eq!(result.unwrap_err().code, ErrorCode::INTERNAL_ERROR);

    // Try to cat a non-ignored file
    let allowed_file_path = temp_dir.path().join("allowed.txt");
    std::fs::write(&allowed_file_path, "allowed content").unwrap();

    let result = router
        .call_tool(
            "shell",
            json!({
                "command": format!("cat {}", allowed_file_path.to_str().unwrap())
            }),
            dummy_sender(),
        )
        .await;

    assert!(result.is_ok(), "Should be able to cat non-ignored file");

    temp_dir.close().unwrap();
}

#[tokio::test]
#[serial]
async fn test_gitignore_fallback_when_no_gooseignore() {
    let temp_dir = tempfile::tempdir().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    // Create a .gitignore file but no .gooseignore
    std::fs::write(temp_dir.path().join(".gitignore"), "*.log\n*.tmp\n.env").unwrap();

    let router = DeveloperRouter::new();

    // Test that gitignore patterns are respected
    assert!(
        router.is_ignored(Path::new("test.log")),
        "*.log pattern from .gitignore should be ignored"
    );
    assert!(
        router.is_ignored(Path::new("build.tmp")),
        "*.tmp pattern from .gitignore should be ignored"
    );
    assert!(
        router.is_ignored(Path::new(".env")),
        ".env pattern from .gitignore should be ignored"
    );
    assert!(
        !router.is_ignored(Path::new("test.txt")),
        "test.txt should not be ignored"
    );

    temp_dir.close().unwrap();
}

#[tokio::test]
#[serial]
async fn test_gooseignore_takes_precedence_over_gitignore() {
    let temp_dir = tempfile::tempdir().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    // Create both .gooseignore and .gitignore files with different patterns
    std::fs::write(temp_dir.path().join(".gooseignore"), "*.secret").unwrap();
    std::fs::write(temp_dir.path().join(".gitignore"), "*.log\ntarget/").unwrap();

    let router = DeveloperRouter::new();

    // .gooseignore patterns should be used
    assert!(
        router.is_ignored(Path::new("test.secret")),
        "*.secret pattern from .gooseignore should be ignored"
    );

    // .gitignore patterns should NOT be used when .gooseignore exists
    assert!(
        !router.is_ignored(Path::new("test.log")),
        "*.log pattern from .gitignore should NOT be ignored when .gooseignore exists"
    );
    assert!(
        !router.is_ignored(Path::new("build.tmp")),
        "*.tmp pattern from .gitignore should NOT be ignored when .gooseignore exists"
    );

    temp_dir.close().unwrap();
}

#[tokio::test]
#[serial]
async fn test_default_patterns_when_no_ignore_files() {
    let temp_dir = tempfile::tempdir().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    // Don't create any ignore files
    let router = DeveloperRouter::new();

    // Default patterns should be used
    assert!(
        router.is_ignored(Path::new(".env")),
        ".env should be ignored by default patterns"
    );
    assert!(
        router.is_ignored(Path::new(".env.local")),
        ".env.local should be ignored by default patterns"
    );
    assert!(
        router.is_ignored(Path::new("secrets.txt")),
        "secrets.txt should be ignored by default patterns"
    );
    assert!(
        !router.is_ignored(Path::new("normal.txt")),
        "normal.txt should not be ignored"
    );

    temp_dir.close().unwrap();
}

#[tokio::test]
#[serial]
async fn test_text_editor_descriptions() {
    let temp_dir = tempfile::tempdir().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    // Test without editor API configured (should be the case in tests due to cfg!(test))
    let router = DeveloperRouter::new();
    let tools = router.list_tools();
    let text_editor_tool = tools.iter().find(|t| t.name == "text_editor").unwrap();

    // Should use traditional description with str_replace command
    assert!(text_editor_tool
        .description
        .as_ref()
        .is_some_and(|desc| desc.contains("Replace a string in a file with a new string")));
    assert!(text_editor_tool
        .description
        .as_ref()
        .is_some_and(|desc| desc.contains("str_replace")));

    // Should not contain editor API description or edit_file command
    assert!(!text_editor_tool
        .description
        .as_ref()
        .is_some_and(|desc| desc.contains("Edit the file with the new content")));
    assert!(!text_editor_tool
        .description
        .as_ref()
        .is_some_and(|desc| desc.contains("edit_file")));
    assert!(!text_editor_tool
        .description
        .as_ref()
        .is_some_and(|desc| desc.contains("work out how to place old_str with it intelligently")));

    temp_dir.close().unwrap();
}

#[tokio::test]
#[serial]
async fn test_text_editor_respects_gitignore_fallback() {
    let temp_dir = tempfile::tempdir().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    // Create a .gitignore file but no .gooseignore
    std::fs::write(temp_dir.path().join(".gitignore"), "*.log").unwrap();

    let router = DeveloperRouter::new();

    // Try to write to a file ignored by .gitignore
    let result = router
        .call_tool(
            "text_editor",
            json!({
                "command": "write",
                "path": temp_dir.path().join("test.log").to_str().unwrap(),
                "file_text": "test content"
            }),
            dummy_sender(),
        )
        .await;

    assert!(
        result.is_err(),
        "Should not be able to write to file ignored by .gitignore fallback"
    );
    assert_eq!(result.unwrap_err().code, ErrorCode::INTERNAL_ERROR);

    // Try to write to a non-ignored file
    let result = router
        .call_tool(
            "text_editor",
            json!({
                "command": "write",
                "path": temp_dir.path().join("allowed.txt").to_str().unwrap(),
                "file_text": "test content"
            }),
            dummy_sender(),
        )
        .await;

    assert!(
        result.is_ok(),
        "Should be able to write to non-ignored file"
    );

    temp_dir.close().unwrap();
}

#[tokio::test]
#[serial]
async fn test_bash_respects_gitignore_fallback() {
    let temp_dir = tempfile::tempdir().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    // Create a .gitignore file but no .gooseignore
    std::fs::write(temp_dir.path().join(".gitignore"), "*.log").unwrap();

    let router = DeveloperRouter::new();

    // Create a file that would be ignored by .gitignore
    let log_file_path = temp_dir.path().join("test.log");
    std::fs::write(&log_file_path, "log content").unwrap();

    // Try to cat the ignored file
    let result = router
        .call_tool(
            "shell",
            json!({
                "command": format!("cat {}", log_file_path.to_str().unwrap())
            }),
            dummy_sender(),
        )
        .await;

    assert!(
        result.is_err(),
        "Should not be able to cat file ignored by .gitignore fallback"
    );
    assert_eq!(result.unwrap_err().code, ErrorCode::INTERNAL_ERROR);

    // Try to cat a non-ignored file
    let allowed_file_path = temp_dir.path().join("allowed.txt");
    std::fs::write(&allowed_file_path, "allowed content").unwrap();

    let result = router
        .call_tool(
            "shell",
            json!({
                "command": format!("cat {}", allowed_file_path.to_str().unwrap())
            }),
            dummy_sender(),
        )
        .await;

    assert!(result.is_ok(), "Should be able to cat non-ignored file");

    temp_dir.close().unwrap();
}

// Tests for view_range functionality
#[tokio::test]
#[serial]
async fn test_text_editor_view_range() {
    let router = get_router().await;

    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    let file_path_str = file_path.to_str().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    // Create a multi-line file
    let content = "Line 1\nLine 2\nLine 3\nLine 4\nLine 5\nLine 6\nLine 7\nLine 8\nLine 9\nLine 10";
    router
        .call_tool(
            "text_editor",
            json!({
                "command": "write",
                "path": file_path_str,
                "file_text": content
            }),
            dummy_sender(),
        )
        .await
        .unwrap();

    // Test viewing specific range
    let view_result = router
        .call_tool(
            "text_editor",
            json!({
                "command": "view",
                "path": file_path_str,
                "view_range": [3, 6]
            }),
            dummy_sender(),
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

    // Should contain lines 3-6 with line numbers
    assert!(text.text.contains("3: Line 3"));
    assert!(text.text.contains("4: Line 4"));
    assert!(text.text.contains("5: Line 5"));
    assert!(text.text.contains("6: Line 6"));
    assert!(text.text.contains("(lines 3-6)"));
    // Should not contain other lines
    assert!(!text.text.contains("1: Line 1"));
    assert!(!text.text.contains("7: Line 7"));

    temp_dir.close().unwrap();
}

#[tokio::test]
#[serial]
async fn test_text_editor_view_range_to_end() {
    let router = get_router().await;

    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    let file_path_str = file_path.to_str().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    // Create a multi-line file
    let content = "Line 1\nLine 2\nLine 3\nLine 4\nLine 5";
    router
        .call_tool(
            "text_editor",
            json!({
                "command": "write",
                "path": file_path_str,
                "file_text": content
            }),
            dummy_sender(),
        )
        .await
        .unwrap();

    // Test viewing from line 3 to end using -1
    let view_result = router
        .call_tool(
            "text_editor",
            json!({
                "command": "view",
                "path": file_path_str,
                "view_range": [3, -1]
            }),
            dummy_sender(),
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

    // Should contain lines 3 to end
    assert!(text.text.contains("3: Line 3"));
    assert!(text.text.contains("4: Line 4"));
    assert!(text.text.contains("5: Line 5"));
    assert!(text.text.contains("(lines 3-end)"));
    // Should not contain earlier lines
    assert!(!text.text.contains("1: Line 1"));
    assert!(!text.text.contains("2: Line 2"));

    temp_dir.close().unwrap();
}

#[tokio::test]
#[serial]
async fn test_text_editor_view_range_invalid() {
    let router = get_router().await;

    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    let file_path_str = file_path.to_str().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    // Create a small file
    let content = "Line 1\nLine 2\nLine 3";
    router
        .call_tool(
            "text_editor",
            json!({
                "command": "write",
                "path": file_path_str,
                "file_text": content
            }),
            dummy_sender(),
        )
        .await
        .unwrap();

    // Test invalid range - start beyond end of file
    let result = router
        .call_tool(
            "text_editor",
            json!({
                "command": "view",
                "path": file_path_str,
                "view_range": [10, 15]
            }),
            dummy_sender(),
        )
        .await;

    assert!(result.is_err());
    let err = result.err().unwrap();
    assert_eq!(err.code, ErrorCode::INVALID_PARAMS);
    assert!(err.to_string().contains("beyond the end of the file"));

    // Test invalid range - start >= end
    let result = router
        .call_tool(
            "text_editor",
            json!({
                "command": "view",
                "path": file_path_str,
                "view_range": [3, 2]
            }),
            dummy_sender(),
        )
        .await;

    assert!(result.is_err());
    let err = result.err().unwrap();
    assert_eq!(err.code, ErrorCode::INVALID_PARAMS);
    assert!(err.to_string().contains("must be less than end line"));

    temp_dir.close().unwrap();
}

// Tests for insert functionality
#[tokio::test]
#[serial]
async fn test_text_editor_insert_at_beginning() {
    let router = get_router().await;

    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    let file_path_str = file_path.to_str().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    // Create a file with some content
    let content = "Line 2\nLine 3\nLine 4";
    router
        .call_tool(
            "text_editor",
            json!({
                "command": "write",
                "path": file_path_str,
                "file_text": content
            }),
            dummy_sender(),
        )
        .await
        .unwrap();

    // Insert at the beginning (line 0)
    let insert_result = router
        .call_tool(
            "text_editor",
            json!({
                "command": "insert",
                "path": file_path_str,
                "insert_line": 0,
                "new_str": "Line 1"
            }),
            dummy_sender(),
        )
        .await
        .unwrap();

    let text = insert_result
        .iter()
        .find(|c| {
            c.audience()
                .is_some_and(|roles| roles.contains(&Role::Assistant))
        })
        .unwrap()
        .as_text()
        .unwrap();

    assert!(text.text.contains("Text has been inserted at line 1"));

    // Verify the file content
    let view_result = router
        .call_tool(
            "text_editor",
            json!({
                "command": "view",
                "path": file_path_str
            }),
            dummy_sender(),
        )
        .await
        .unwrap();

    let view_text = view_result
        .iter()
        .find(|c| {
            c.audience()
                .is_some_and(|roles| roles.contains(&Role::User))
        })
        .unwrap()
        .as_text()
        .unwrap();

    assert!(view_text.text.contains("1: Line 1"));
    assert!(view_text.text.contains("2: Line 2"));
    assert!(view_text.text.contains("3: Line 3"));
    assert!(view_text.text.contains("4: Line 4"));

    temp_dir.close().unwrap();
}

#[tokio::test]
#[serial]
async fn test_text_editor_insert_in_middle() {
    let router = get_router().await;

    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    let file_path_str = file_path.to_str().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    // Create a file with some content
    let content = "Line 1\nLine 2\nLine 4\nLine 5";
    router
        .call_tool(
            "text_editor",
            json!({
                "command": "write",
                "path": file_path_str,
                "file_text": content
            }),
            dummy_sender(),
        )
        .await
        .unwrap();

    // Insert after line 2
    let insert_result = router
        .call_tool(
            "text_editor",
            json!({
                "command": "insert",
                "path": file_path_str,
                "insert_line": 2,
                "new_str": "Line 3"
            }),
            dummy_sender(),
        )
        .await
        .unwrap();

    let text = insert_result
        .iter()
        .find(|c| {
            c.audience()
                .is_some_and(|roles| roles.contains(&Role::Assistant))
        })
        .unwrap()
        .as_text()
        .unwrap();

    assert!(text.text.contains("Text has been inserted at line 3"));

    // Verify the file content
    let view_result = router
        .call_tool(
            "text_editor",
            json!({
                "command": "view",
                "path": file_path_str
            }),
            dummy_sender(),
        )
        .await
        .unwrap();

    let view_text = view_result
        .iter()
        .find(|c| {
            c.audience()
                .is_some_and(|roles| roles.contains(&Role::User))
        })
        .unwrap()
        .as_text()
        .unwrap();

    assert!(view_text.text.contains("1: Line 1"));
    assert!(view_text.text.contains("2: Line 2"));
    assert!(view_text.text.contains("3: Line 3"));
    assert!(view_text.text.contains("4: Line 4"));
    assert!(view_text.text.contains("5: Line 5"));

    temp_dir.close().unwrap();
}

#[tokio::test]
#[serial]
async fn test_text_editor_insert_at_end() {
    let router = get_router().await;

    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    let file_path_str = file_path.to_str().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    // Create a file with some content
    let content = "Line 1\nLine 2\nLine 3";
    router
        .call_tool(
            "text_editor",
            json!({
                "command": "write",
                "path": file_path_str,
                "file_text": content
            }),
            dummy_sender(),
        )
        .await
        .unwrap();

    // Insert at the end (after line 3)
    let insert_result = router
        .call_tool(
            "text_editor",
            json!({
                "command": "insert",
                "path": file_path_str,
                "insert_line": 3,
                "new_str": "Line 4"
            }),
            dummy_sender(),
        )
        .await
        .unwrap();

    let text = insert_result
        .iter()
        .find(|c| {
            c.audience()
                .is_some_and(|roles| roles.contains(&Role::Assistant))
        })
        .unwrap()
        .as_text()
        .unwrap();

    assert!(text.text.contains("Text has been inserted at line 4"));

    // Verify the file content
    let view_result = router
        .call_tool(
            "text_editor",
            json!({
                "command": "view",
                "path": file_path_str
            }),
            dummy_sender(),
        )
        .await
        .unwrap();

    let view_text = view_result
        .iter()
        .find(|c| {
            c.audience()
                .is_some_and(|roles| roles.contains(&Role::User))
        })
        .unwrap()
        .as_text()
        .unwrap();

    assert!(view_text.text.contains("1: Line 1"));
    assert!(view_text.text.contains("2: Line 2"));
    assert!(view_text.text.contains("3: Line 3"));
    assert!(view_text.text.contains("4: Line 4"));

    temp_dir.close().unwrap();
}

#[tokio::test]
#[serial]
async fn test_text_editor_insert_at_end_negative() {
    let router = get_router().await;

    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    let file_path_str = file_path.to_str().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    // Create a file with some content
    let content = "Line 1\nLine 2\nLine 3";
    router
        .call_tool(
            "text_editor",
            json!({
                "command": "write",
                "path": file_path_str,
                "file_text": content
            }),
            dummy_sender(),
        )
        .await
        .unwrap();

    // Insert at the end (after line 3)
    let insert_result = router
        .call_tool(
            "text_editor",
            json!({
                "command": "insert",
                "path": file_path_str,
                "insert_line": -1,
                "new_str": "Line 4"
            }),
            dummy_sender(),
        )
        .await
        .unwrap();

    let text = insert_result
        .iter()
        .find(|c| {
            c.audience()
                .is_some_and(|roles| roles.contains(&Role::Assistant))
        })
        .unwrap()
        .as_text()
        .unwrap();

    assert!(text.text.contains("Text has been inserted at line 4"));

    // Verify the file content
    let view_result = router
        .call_tool(
            "text_editor",
            json!({
                "command": "view",
                "path": file_path_str
            }),
            dummy_sender(),
        )
        .await
        .unwrap();

    let view_text = view_result
        .iter()
        .find(|c| {
            c.audience()
                .is_some_and(|roles| roles.contains(&Role::User))
        })
        .unwrap()
        .as_text()
        .unwrap();

    assert!(view_text.text.contains("1: Line 1"));
    assert!(view_text.text.contains("2: Line 2"));
    assert!(view_text.text.contains("3: Line 3"));
    assert!(view_text.text.contains("4: Line 4"));

    temp_dir.close().unwrap();
}

#[tokio::test]
#[serial]
async fn test_text_editor_insert_invalid_line() {
    let router = get_router().await;

    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    let file_path_str = file_path.to_str().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    // Create a file with some content
    let content = "Line 1\nLine 2\nLine 3";
    router
        .call_tool(
            "text_editor",
            json!({
                "command": "write",
                "path": file_path_str,
                "file_text": content
            }),
            dummy_sender(),
        )
        .await
        .unwrap();

    // Try to insert beyond the end of the file
    let result = router
        .call_tool(
            "text_editor",
            json!({
                "command": "insert",
                "path": file_path_str,
                "insert_line": 10,
                "new_str": "Line 11"
            }),
            dummy_sender(),
        )
        .await;

    assert!(result.is_err());
    let err = result.err().unwrap();
    assert_eq!(err.code, ErrorCode::INVALID_PARAMS);
    assert!(err.to_string().contains("beyond the end of the file"));

    temp_dir.close().unwrap();
}

#[tokio::test]
#[serial]
async fn test_text_editor_insert_missing_parameters() {
    let router = get_router().await;

    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    let file_path_str = file_path.to_str().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    // Create a file
    router
        .call_tool(
            "text_editor",
            json!({
                "command": "write",
                "path": file_path_str,
                "file_text": "Test content"
            }),
            dummy_sender(),
        )
        .await
        .unwrap();

    // Try insert without insert_line parameter
    let result = router
        .call_tool(
            "text_editor",
            json!({
                "command": "insert",
                "path": file_path_str,
                "new_str": "New line"
            }),
            dummy_sender(),
        )
        .await;

    assert!(result.is_err());
    let err = result.err().unwrap();
    assert_eq!(err.code, ErrorCode::INVALID_PARAMS);
    assert!(err.to_string().contains("Missing 'insert_line' parameter"));

    // Try insert without new_str parameter
    let result = router
        .call_tool(
            "text_editor",
            json!({
                "command": "insert",
                "path": file_path_str,
                "insert_line": 1
            }),
            dummy_sender(),
        )
        .await;

    assert!(result.is_err());
    let err = result.err().unwrap();
    assert_eq!(err.code, ErrorCode::INVALID_PARAMS);
    assert!(err.to_string().contains("Missing 'new_str' parameter"));

    temp_dir.close().unwrap();
}

#[tokio::test]
#[serial]
async fn test_text_editor_insert_with_undo() {
    let router = get_router().await;

    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    let file_path_str = file_path.to_str().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    // Create a file with some content
    let content = "Line 1\nLine 2";
    router
        .call_tool(
            "text_editor",
            json!({
                "command": "write",
                "path": file_path_str,
                "file_text": content
            }),
            dummy_sender(),
        )
        .await
        .unwrap();

    // Insert a line
    router
        .call_tool(
            "text_editor",
            json!({
                "command": "insert",
                "path": file_path_str,
                "insert_line": 1,
                "new_str": "Inserted Line"
            }),
            dummy_sender(),
        )
        .await
        .unwrap();

    // Undo the insert
    let undo_result = router
        .call_tool(
            "text_editor",
            json!({
                "command": "undo_edit",
                "path": file_path_str
            }),
            dummy_sender(),
        )
        .await
        .unwrap();

    let text = undo_result.first().unwrap().as_text().unwrap();
    assert!(text.text.contains("Undid the last edit"));

    // Verify the file is back to original content
    let view_result = router
        .call_tool(
            "text_editor",
            json!({
                "command": "view",
                "path": file_path_str
            }),
            dummy_sender(),
        )
        .await
        .unwrap();

    let view_text = view_result
        .iter()
        .find(|c| {
            c.audience()
                .is_some_and(|roles| roles.contains(&Role::User))
        })
        .unwrap()
        .as_text()
        .unwrap();

    assert!(view_text.text.contains("1: Line 1"));
    assert!(view_text.text.contains("2: Line 2"));
    assert!(!view_text.text.contains("Inserted Line"));

    temp_dir.close().unwrap();
}

#[tokio::test]
#[serial]
async fn test_text_editor_insert_nonexistent_file() {
    let router = get_router().await;

    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("nonexistent.txt");
    let file_path_str = file_path.to_str().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    // Try to insert into a nonexistent file
    let result = router
        .call_tool(
            "text_editor",
            json!({
                "command": "insert",
                "path": file_path_str,
                "insert_line": 0,
                "new_str": "New line"
            }),
            dummy_sender(),
        )
        .await;

    assert!(result.is_err());
    let err = result.err().unwrap();
    assert_eq!(err.code, ErrorCode::INVALID_PARAMS);
    assert!(err.to_string().contains("does not exist"));

    temp_dir.close().unwrap();
}

#[tokio::test]
#[serial]
async fn test_text_editor_view_large_file_without_range() {
    let router = get_router().await;

    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("large_file.txt");
    let file_path_str = file_path.to_str().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    // Create a file with more than LINE_READ_LIMIT lines
    let mut content = String::new();
    for i in 1..=LINE_READ_LIMIT + 1 {
        content.push_str(&format!("Line {}\n", i));
    }

    router
        .call_tool(
            "text_editor",
            json!({
                "command": "write",
                "path": file_path_str,
                "file_text": content
            }),
            dummy_sender(),
        )
        .await
        .unwrap();

    // Test viewing without view_range - should trigger the error
    let result = router
        .call_tool(
            "text_editor",
            json!({
                "command": "view",
                "path": file_path_str
            }),
            dummy_sender(),
        )
        .await;

    assert!(result.is_err());
    let err = result.err().unwrap();
    assert_eq!(err.code, ErrorCode::INTERNAL_ERROR);
    assert!(err.to_string().contains("2001 lines long"));
    assert!(err
        .to_string()
        .contains("recommended to read in with view_range"));
    assert!(err
        .to_string()
        .contains("please pass in view_range with [1, 2001]"));

    // Test viewing with view_range - should work
    let result = router
        .call_tool(
            "text_editor",
            json!({
                "command": "view",
                "path": file_path_str,
                "view_range": [1, 100]
            }),
            dummy_sender(),
        )
        .await;

    assert!(result.is_ok());
    let view_result = result.unwrap();
    let text = view_result
        .iter()
        .find(|c| {
            c.audience()
                .is_some_and(|roles| roles.contains(&Role::User))
        })
        .unwrap()
        .as_text()
        .unwrap();

    // Should contain lines 1-100
    assert!(text.text.contains("1: Line 1"));
    assert!(text.text.contains("100: Line 100"));
    assert!(!text.text.contains("101: Line 101"));

    // Test viewing with explicit full range - should work
    let result = router
        .call_tool(
            "text_editor",
            json!({
                "command": "view",
                "path": file_path_str,
                "view_range": [1, 2001]
            }),
            dummy_sender(),
        )
        .await;

    assert!(result.is_ok());

    temp_dir.close().unwrap();
}

#[tokio::test]
#[serial]
async fn test_text_editor_view_file_with_exactly_2000_lines() {
    let router = get_router().await;

    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("file_2000.txt");
    let file_path_str = file_path.to_str().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    // Create a file with exactly 2000 lines (should not trigger the check)
    let mut content = String::new();
    for i in 1..=2000 {
        content.push_str(&format!("Line {}\n", i));
    }

    router
        .call_tool(
            "text_editor",
            json!({
                "command": "write",
                "path": file_path_str,
                "file_text": content
            }),
            dummy_sender(),
        )
        .await
        .unwrap();

    // Test viewing without view_range - should work since it's exactly 2000 lines
    let result = router
        .call_tool(
            "text_editor",
            json!({
                "command": "view",
                "path": file_path_str
            }),
            dummy_sender(),
        )
        .await;

    assert!(result.is_ok());
    let view_result = result.unwrap();
    let text = view_result
        .iter()
        .find(|c| {
            c.audience()
                .is_some_and(|roles| roles.contains(&Role::User))
        })
        .unwrap()
        .as_text()
        .unwrap();

    // Should contain all lines
    assert!(text.text.contains("1: Line 1"));
    assert!(text.text.contains("2000: Line 2000"));

    temp_dir.close().unwrap();
}

#[tokio::test]
#[serial]
async fn test_text_editor_view_small_file_without_range() {
    let router = get_router().await;

    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("small_file.txt");
    let file_path_str = file_path.to_str().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    // Create a file with less than 2000 lines
    let mut content = String::new();
    for i in 1..=100 {
        content.push_str(&format!("Line {}\n", i));
    }

    router
        .call_tool(
            "text_editor",
            json!({
                "command": "write",
                "path": file_path_str,
                "file_text": content
            }),
            dummy_sender(),
        )
        .await
        .unwrap();

    // Test viewing without view_range - should work fine
    let result = router
        .call_tool(
            "text_editor",
            json!({
                "command": "view",
                "path": file_path_str
            }),
            dummy_sender(),
        )
        .await;

    assert!(result.is_ok());
    let view_result = result.unwrap();
    let text = view_result
        .iter()
        .find(|c| {
            c.audience()
                .is_some_and(|roles| roles.contains(&Role::User))
        })
        .unwrap()
        .as_text()
        .unwrap();

    // Should contain all lines
    assert!(text.text.contains("1: Line 1"));
    assert!(text.text.contains("100: Line 100"));

    temp_dir.close().unwrap();
}

#[tokio::test]
#[serial]
async fn test_bash_output_truncation() {
    let temp_dir = tempfile::tempdir().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    let router = get_router().await;

    // Create a command that generates > 100 lines of output
    let command = if cfg!(windows) {
        "for /L %i in (1,1,150) do @echo Line %i"
    } else {
        "for i in {1..150}; do echo \"Line $i\"; done"
    };

    let result = router
        .call_tool("shell", json!({ "command": command }), dummy_sender())
        .await
        .unwrap();

    // Should have two Content items
    assert_eq!(result.len(), 2);

    // Find the Assistant and User content
    let assistant_content = result
        .iter()
        .find(|c| {
            c.audience()
                .is_some_and(|roles| roles.contains(&Role::Assistant))
        })
        .unwrap()
        .as_text()
        .unwrap();

    let user_content = result
        .iter()
        .find(|c| {
            c.audience()
                .is_some_and(|roles| roles.contains(&Role::User))
        })
        .unwrap()
        .as_text()
        .unwrap();

    // Assistant should get the full message with temp file info
    assert!(assistant_content.text.contains("private note: output was"));

    // User should only get the truncated output with prefix
    assert!(user_content.text.starts_with("..."));
    assert!(!user_content.text.contains("private note: output was"));

    // User output should contain lines 51-150 (last 100 lines)
    assert!(user_content.text.contains("Line 51"));
    assert!(user_content.text.contains("Line 150"));
    assert!(!user_content.text.contains("Line 50"));

    let start_tag = "remainder of lines in";
    let end_tag = "do not show tmp file to user";

    if let (Some(start), Some(end)) = (
        assistant_content.text.find(start_tag),
        assistant_content.text.find(end_tag),
    ) {
        let start_idx = start + start_tag.len();
        if start_idx < end {
            let path = assistant_content.text[start_idx..end].trim();
            println!("Extracted path: {}", path);

            let file_contents = read_to_string(path).expect("Failed to read extracted temp file");

            let lines: Vec<&str> = file_contents.lines().collect();

            // Ensure we have exactly 150 lines
            assert_eq!(lines.len(), 150, "Expected 150 lines in temp file");

            // Ensure the first and last lines are correct
            assert_eq!(lines.first(), Some(&"Line 1"), "First line mismatch");
            assert_eq!(lines.last(), Some(&"Line 150"), "Last line mismatch");
        } else {
            panic!("No path found in bash output truncation output");
        }
    } else {
        panic!("Failed to find start or end tag in bash output truncation output");
    }

    temp_dir.close().unwrap();
}

#[test]
#[serial]
fn test_process_shell_output_short() {
    let dir = TempDir::new().unwrap();
    std::env::set_current_dir(dir.path()).unwrap();

    let router = DeveloperRouter::new();

    // Test with short output (< 100 lines)
    let short_output = "Line 1\nLine 2\nLine 3\nLine 4\nLine 5";
    let result = router.process_shell_output(short_output).unwrap();

    // Both outputs should be the same for short outputs
    assert_eq!(result.0, short_output);
    assert_eq!(result.1, short_output);
}

#[test]
#[serial]
fn test_process_shell_output_empty() {
    let dir = TempDir::new().unwrap();
    std::env::set_current_dir(dir.path()).unwrap();

    let router = DeveloperRouter::new();

    // Test with empty output
    let empty_output = "";
    let result = router.process_shell_output(empty_output).unwrap();

    // Both outputs should be empty
    assert_eq!(result.0, "");
    assert_eq!(result.1, "");
}

#[tokio::test]
#[serial]
async fn test_shell_output_without_trailing_newline() {
    let temp_dir = tempfile::tempdir().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    let router = get_router().await;

    // Test command that outputs content without a trailing newline
    let command = if cfg!(windows) {
        "echo|set /p=\"Content without newline\""
    } else {
        "printf 'Content without newline'"
    };

    let result = router
        .call_tool("shell", json!({ "command": command }), dummy_sender())
        .await
        .unwrap();

    // Find the assistant content (which contains the full output)
    let assistant_content = result
        .iter()
        .find(|c| {
            c.audience()
                .is_some_and(|roles| roles.contains(&Role::Assistant))
        })
        .unwrap()
        .as_text()
        .unwrap();

    // The output should contain the content even without a trailing newline
    assert!(
        assistant_content.text.contains("Content without newline"),
        "Output should contain content even without trailing newline, but got: {}",
        assistant_content.text
    );

    temp_dir.close().unwrap();
}

#[tokio::test]
#[serial]
async fn test_search_code_file_search() {
    // Skip test if ripgrep is not available
    if std::process::Command::new("rg")
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!("Skipping test: ripgrep not installed");
        return;
    }

    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test_file.rs");
    std::fs::write(&test_file, "fn main() {\n    println!(\"Hello\");\n}").unwrap();

    let another_file = temp_dir.path().join("another.txt");
    std::fs::write(&another_file, "Some text content").unwrap();

    std::env::set_current_dir(&temp_dir).unwrap();

    let router = get_router().await;

    // Test file search - use "." to match any file with test_file in the name
    let result = router
        .call_tool(
            "search_code",
            json!({
                "search_type": "file",
                "search_terms": ["."]
            }),
            dummy_sender(),
        )
        .await;

    if result.is_err() {
        eprintln!("Error calling search_code: {:?}", result.as_ref().err());
    }
    assert!(result.is_ok());
    let response = result.unwrap();

    // Find the text content in the response
    let content = response
        .iter()
        .find(|c| {
            c.audience()
                .is_some_and(|roles| roles.contains(&Role::User))
        })
        .unwrap()
        .as_text()
        .unwrap();

    assert!(
        content.text.contains("test_file.rs"),
        "Should find test_file.rs, but got: {}",
        content.text
    );

    temp_dir.close().unwrap();
}

#[tokio::test]
#[serial]
async fn test_search_code_content_search() {
    // Skip test if ripgrep is not available
    if std::process::Command::new("rg")
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!("Skipping test: ripgrep not installed");
        return;
    }

    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("example.rs");
    std::fs::write(
        &test_file,
        "fn greet() {\n    println!(\"Hello, World!\");\n}\n\nfn main() {\n    greet();\n}",
    )
    .unwrap();

    let another_file = temp_dir.path().join("lib.rs");
    std::fs::write(&another_file, "pub fn calculate() -> i32 {\n    42\n}").unwrap();

    std::env::set_current_dir(&temp_dir).unwrap();

    let router = get_router().await;

    // Test content search with context
    let result = router
        .call_tool(
            "search_code",
            json!({
                "search_type": "content",
                "search_terms": ["greet", "calculate"],
                "context_lines": 1
            }),
            dummy_sender(),
        )
        .await;

    if result.is_err() {
        eprintln!("Error calling search_code: {:?}", result.as_ref().err());
    }
    assert!(result.is_ok());
    let response = result.unwrap();

    // Find the text content in the response
    let content = response
        .iter()
        .find(|c| {
            c.audience()
                .is_some_and(|roles| roles.contains(&Role::User))
        })
        .unwrap()
        .as_text()
        .unwrap();

    // Check that both search results are included
    assert!(
        content.text.contains("greet"),
        "Should find 'greet' function, but got: {}",
        content.text
    );

    assert!(
        content.text.contains("calculate"),
        "Should find 'calculate' function, but got: {}",
        content.text
    );

    assert!(
        content.text.contains("example.rs") || content.text.contains("lib.rs"),
        "Should contain file names in results, but got: {}",
        content.text
    );

    temp_dir.close().unwrap();
}
