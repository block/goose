use goose_tui::at_mention::{consume_path, process};
use tempfile::TempDir;

// ============================================================================
// process (end-to-end) tests
// ============================================================================

#[test]
fn end_to_end_file_attachment() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("test.rs"), "fn main() {}").unwrap();

    let result = process("Review @test.rs please", dir.path());

    assert!(result.errors.is_empty());
    assert_eq!(result.attachments.len(), 1);
    assert_eq!(result.attachments[0].content, "fn main() {}");
    assert!(result.augmented_text.contains("<attached_files_goose_tui>"));
    assert!(result.augmented_text.ends_with("Review @test.rs please"));
}

#[test]
fn process_missing_file_returns_empty_attachments() {
    let dir = TempDir::new().unwrap();

    let missing = process("Check @nonexistent.txt", dir.path());

    assert!(missing.errors.is_empty());
    assert!(missing.attachments.is_empty());
    assert_eq!(missing.augmented_text, "Check @nonexistent.txt");
}

#[test]
fn process_no_mentions_returns_original() {
    let dir = TempDir::new().unwrap();

    let plain = process("No mentions here", dir.path());

    assert_eq!(plain.augmented_text, "No mentions here");
    assert!(plain.attachments.is_empty());
}

// ============================================================================
// consume_path tests
// ============================================================================

#[test]
fn consume_path_handles_backslash_escape() {
    let input = r"my\ file.txt";
    let mut chars = input.chars().peekable();

    let result = consume_path(&mut chars);

    assert_eq!(result, "my file.txt");
}

#[test]
fn consume_path_handles_multiple_escapes() {
    let input = r"a\ b\ c.txt";
    let mut chars = input.chars().peekable();

    let result = consume_path(&mut chars);

    assert_eq!(result, "a b c.txt");
}

#[test]
fn consume_path_stops_at_terminators() {
    let input = "file.txt,other";
    let mut chars = input.chars().peekable();

    let result = consume_path(&mut chars);

    assert_eq!(result, "file.txt");
    // Verify the comma is still in the iterator
    assert_eq!(chars.next(), Some(','));
}

#[test]
fn consume_path_handles_trailing_backslash() {
    let input = r"file\";
    let mut chars = input.chars().peekable();

    let result = consume_path(&mut chars);

    // Trailing backslash with nothing after should just be consumed
    assert_eq!(result, "file");
}

#[test]
fn consume_path_stops_at_whitespace() {
    let input = "path/to/file.rs rest of text";
    let mut chars = input.chars().peekable();

    let result = consume_path(&mut chars);

    assert_eq!(result, "path/to/file.rs");
}

#[test]
fn consume_path_handles_various_terminators() {
    // Test each terminator from PATH_TERMINATORS: ",:;!?()[]{}\"'"
    let terminators = [
        ',', ':', ';', '!', '?', '(', ')', '[', ']', '{', '}', '"', '\'',
    ];

    for term in terminators {
        let input = format!("file.txt{term}rest");
        let mut chars = input.chars().peekable();

        let result = consume_path(&mut chars);

        assert_eq!(result, "file.txt", "Failed for terminator: {term}");
    }
}

#[test]
fn consume_path_handles_empty_input() {
    let input = "";
    let mut chars = input.chars().peekable();

    let result = consume_path(&mut chars);

    assert_eq!(result, "");
}

#[test]
fn consume_path_handles_only_whitespace() {
    let input = " file.txt";
    let mut chars = input.chars().peekable();

    let result = consume_path(&mut chars);

    // Should stop immediately at whitespace
    assert_eq!(result, "");
}

#[test]
fn consume_path_preserves_slashes() {
    let input = "path/to/nested/file.rs";
    let mut chars = input.chars().peekable();

    let result = consume_path(&mut chars);

    assert_eq!(result, "path/to/nested/file.rs");
}

#[test]
fn consume_path_handles_tilde_home() {
    let input = "~/Documents/file.txt";
    let mut chars = input.chars().peekable();

    let result = consume_path(&mut chars);

    assert_eq!(result, "~/Documents/file.txt");
}
