use goose_tui::at_mention::{consume_path, process};
use tempfile::TempDir;

#[test]
fn process_attaches_file_content() {
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
fn process_ignores_missing_files() {
    let dir = TempDir::new().unwrap();

    let result = process("Check @nonexistent.txt", dir.path());

    assert!(result.attachments.is_empty());
    assert_eq!(result.augmented_text, "Check @nonexistent.txt");
}

#[test]
fn process_returns_original_without_mentions() {
    let dir = TempDir::new().unwrap();

    let result = process("No mentions here", dir.path());

    assert_eq!(result.augmented_text, "No mentions here");
    assert!(result.attachments.is_empty());
}

#[test]
fn consume_path_unescapes_backslash_spaces() {
    let mut chars = r"my\ file.txt".chars().peekable();
    assert_eq!(consume_path(&mut chars), "my file.txt");

    let mut chars = r"a\ b\ c.txt".chars().peekable();
    assert_eq!(consume_path(&mut chars), "a b c.txt");
}

#[test]
fn consume_path_stops_at_terminators() {
    let mut chars = "file.txt,other".chars().peekable();
    assert_eq!(consume_path(&mut chars), "file.txt");
    assert_eq!(chars.next(), Some(','));
}

#[test]
fn consume_path_handles_trailing_backslash() {
    let mut chars = r"file\".chars().peekable();
    assert_eq!(consume_path(&mut chars), "file");
}

#[test]
fn consume_path_stops_at_whitespace() {
    let mut chars = "path/to/file.rs rest".chars().peekable();
    assert_eq!(consume_path(&mut chars), "path/to/file.rs");
}

#[test]
fn consume_path_handles_all_terminators() {
    for term in [
        ',', ':', ';', '!', '?', '(', ')', '[', ']', '{', '}', '"', '\'',
    ] {
        let input = format!("file.txt{term}rest");
        let mut chars = input.chars().peekable();
        assert_eq!(consume_path(&mut chars), "file.txt", "Failed for: {term}");
    }
}

#[test]
fn consume_path_returns_empty_for_empty_or_whitespace() {
    let mut chars = "".chars().peekable();
    assert_eq!(consume_path(&mut chars), "");

    let mut chars = " file.txt".chars().peekable();
    assert_eq!(consume_path(&mut chars), "");
}

#[test]
fn consume_path_preserves_path_structure() {
    let mut chars = "path/to/nested/file.rs".chars().peekable();
    assert_eq!(consume_path(&mut chars), "path/to/nested/file.rs");

    let mut chars = "~/Documents/file.txt".chars().peekable();
    assert_eq!(consume_path(&mut chars), "~/Documents/file.txt");
}
