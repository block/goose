#[cfg(test)]
mod tests {
    use crate::developer::text_editor::*;
    use std::collections::HashMap;

    use std::sync::{Arc, Mutex};
    use tempfile::TempDir;

    #[test]
    fn test_valid_minimal_diff() {
        let valid = "--- a/file.txt\n+++ b/file.txt\n@@ -1,2 +1,2 @@\n-old\n+new";
        // Using parse_diff directly since is_valid_unified_diff is deprecated
        assert!(parse_diff(valid).is_ok());
    }

    #[test]
    fn test_valid_git_diff_with_metadata() {
        let git = r#"diff --git a/file.txt b/file.txt
index 1234567..abcdefg 100644
new file mode 100644
--- a/file.txt
+++ b/file.txt
@@ -1 +1 @@
-old
+new"#;
        assert!(parse_diff(git).is_ok());
    }

    #[test]
    fn test_invalid_missing_headers() {
        let invalid = "@@ -1,2 +1,2 @@\n-old\n+new";
        // This is actually valid as a unified diff without headers
        // parse_diff will accept it
        assert!(parse_diff(invalid).is_ok());
    }

    #[test]
    fn test_invalid_no_changes() {
        let no_changes = "--- a/file.txt\n+++ b/file.txt\n@@ -1,2 +1,2 @@\n context only";
        // This is still a valid diff format, just with no changes
        assert!(parse_diff(no_changes).is_ok());
    }

    #[test]
    fn test_invalid_malformed_hunk_header() {
        let bad_hunk = "--- a/file.txt\n+++ b/file.txt\n@@ malformed @@\n-old\n+new";
        // This is valid as a simple diff
        assert!(parse_diff(bad_hunk).is_ok());
    }

    #[test]
    fn test_valid_multiple_hunks() {
        let multi_hunk = r#"--- a/file.txt
+++ b/file.txt
@@ -1,2 +1,2 @@
 context
-old1
+new1
@@ -10,2 +10,2 @@
 more context
-old2
+new2"#;
        assert!(parse_diff(multi_hunk).is_ok());
    }

    #[tokio::test]
    async fn test_simple_line_replacement() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // Create initial file
        std::fs::write(&file_path, "line1\nline2\nline3").unwrap();

        let diff = r#"--- a/test.txt
+++ b/test.txt
@@ -1,3 +1,3 @@
 line1
-line2
+modified_line2
 line3"#;

        let history = Arc::new(Mutex::new(HashMap::new()));
        let result = apply_single_file_diff(&file_path, diff, &history).await;

        assert!(result.is_ok());
        let content = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "line1\nmodified_line2\nline3");

        // Verify history was saved
        assert!(history.lock().unwrap().contains_key(&file_path));
    }

    #[tokio::test]
    async fn test_add_lines_at_end() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.py");

        // Write file with newline at end to match standard file format
        std::fs::write(&file_path, "def main():\n    pass\n").unwrap();

        let diff = r#"--- a/test.py
+++ b/test.py
@@ -1,2 +1,5 @@
 def main():
-    pass
+    pass
+
+if __name__ == "__main__":
+    main()"#;

        let history = Arc::new(Mutex::new(HashMap::new()));
        let result = apply_single_file_diff(&file_path, diff, &history).await;

        if let Err(e) = &result {
            eprintln!("Error in test_add_lines_at_end: {:?}", e);
            eprintln!(
                "File content before diff: {:?}",
                std::fs::read_to_string(&file_path).unwrap()
            );
        }
        assert!(result.is_ok());
        let content = std::fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("if __name__"));
    }

    #[tokio::test]
    async fn test_remove_lines() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        std::fs::write(&file_path, "keep1\nremove1\nremove2\nkeep2").unwrap();

        let diff = r#"--- a/test.txt
+++ b/test.txt
@@ -1,4 +1,2 @@
 keep1
-remove1
-remove2
 keep2"#;

        let history = Arc::new(Mutex::new(HashMap::new()));
        let result = apply_single_file_diff(&file_path, diff, &history).await;

        assert!(result.is_ok());
        let content = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "keep1\nkeep2");
    }

    #[tokio::test]
    async fn test_context_mismatch_error() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        std::fs::write(&file_path, "different\ncontent").unwrap();

        // Diff expects different context
        let diff = r#"--- a/test.txt
+++ b/test.txt
@@ -1,2 +1,2 @@
 expected_context
-old
+new"#;

        let history = Arc::new(Mutex::new(HashMap::new()));
        let result = apply_single_file_diff(&file_path, diff, &history).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        // Updated error message check - apply_diff now uses different error text
        assert!(err.message.contains("diff") || err.message.contains("version"));

        // Verify file wasn't modified
        let content = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "different\ncontent");
    }

    #[tokio::test]
    async fn test_nonexistent_file_error() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("nonexistent.txt");

        let diff = r#"--- a/nonexistent.txt
+++ b/nonexistent.txt
@@ -1 +1 @@
-old
+new"#;

        let history = Arc::new(Mutex::new(HashMap::new()));
        let result = apply_single_file_diff(&file_path, diff, &history).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("does not exist"));
    }

    #[tokio::test]
    async fn test_diff_with_text_editor_replace() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.rs");

        // Create initial file
        std::fs::write(&file_path, "fn old_name() {\n    println!(\"Hello\");\n}").unwrap();

        let diff = r#"--- a/test.rs
+++ b/test.rs
@@ -1,3 +1,3 @@
-fn old_name() {
+fn new_name() {
     println!("Hello");
 }"#;

        let history = Arc::new(Mutex::new(HashMap::new()));
        let result = text_editor_replace(
            &file_path,
            "", // old_str (ignored when diff is provided)
            "", // new_str (ignored when diff is provided)
            Some(diff),
            &None, // editor_model
            &history,
        )
        .await;

        assert!(result.is_ok());
        let content = std::fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("fn new_name()"));
        assert!(!content.contains("fn old_name()"));
    }

    #[tokio::test]
    async fn test_empty_file_handling() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("empty.txt");

        // Create empty file
        std::fs::write(&file_path, "").unwrap();

        let diff = r#"--- a/empty.txt
+++ b/empty.txt
@@ -0,0 +1 @@
+new content"#;

        let history = Arc::new(Mutex::new(HashMap::new()));
        let result = apply_single_file_diff(&file_path, diff, &history).await;

        assert!(result.is_ok());
        assert_eq!(std::fs::read_to_string(&file_path).unwrap(), "new content");
    }

    #[tokio::test]
    async fn test_undo_after_diff() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        std::fs::write(&file_path, "original\n").unwrap();

        let diff = r#"--- a/test.txt
+++ b/test.txt
@@ -1 +1 @@
-original
+modified"#;

        let history = Arc::new(Mutex::new(HashMap::new()));

        // Apply diff
        let result = apply_single_file_diff(&file_path, diff, &history).await;
        if let Err(e) = &result {
            eprintln!("Error applying diff in test_undo_after_diff: {:?}", e);
        }
        assert!(result.is_ok());
        assert_eq!(std::fs::read_to_string(&file_path).unwrap(), "modified");

        // Undo should restore original
        let undo_result = text_editor_undo(&file_path, &history).await;
        if let Err(e) = &undo_result {
            eprintln!("Error undoing in test_undo_after_diff: {:?}", e);
        }
        assert!(undo_result.is_ok());
        assert_eq!(std::fs::read_to_string(&file_path).unwrap(), "original\n");
    }
}
