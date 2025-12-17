use goose::hints::{
    load_hints_from_directory, AGENTS_MD_FILENAME, DYNAMIC_SUBDIRECTORY_HINT_LOADING_ENV,
};

#[test]
fn test_load_hints_from_directory_enabled_by_default() {
    temp_env::with_var_unset(DYNAMIC_SUBDIRECTORY_HINT_LOADING_ENV, || {
        let temp_dir = tempfile::tempdir().unwrap();
        let subdir = temp_dir.path().join("subdir");
        std::fs::create_dir(&subdir).unwrap();
        let agents_path = subdir.join("AGENTS.md");
        std::fs::write(&agents_path, "Test content").unwrap();

        let result =
            load_hints_from_directory(&subdir, temp_dir.path(), &[AGENTS_MD_FILENAME.to_string()]);
        assert!(result.is_some());
    });
}

#[test]
fn test_load_hints_from_directory_can_be_disabled() {
    temp_env::with_var(DYNAMIC_SUBDIRECTORY_HINT_LOADING_ENV, Some("false"), || {
        let temp_dir = tempfile::tempdir().unwrap();
        let subdir = temp_dir.path().join("subdir");
        std::fs::create_dir(&subdir).unwrap();
        let agents_path = subdir.join("AGENTS.md");
        std::fs::write(&agents_path, "Test content").unwrap();

        let result =
            load_hints_from_directory(&subdir, temp_dir.path(), &[AGENTS_MD_FILENAME.to_string()]);
        assert!(result.is_none());
    });
}

#[test]
fn test_load_hints_from_directory_enabled() {
    temp_env::with_var(DYNAMIC_SUBDIRECTORY_HINT_LOADING_ENV, Some("true"), || {
        let temp_dir = tempfile::tempdir().unwrap();
        let subdir = temp_dir.path().join("subdir");
        std::fs::create_dir(&subdir).unwrap();
        let agents_path = subdir.join("AGENTS.md");
        std::fs::write(&agents_path, "Test content").unwrap();

        let result =
            load_hints_from_directory(&subdir, temp_dir.path(), &[AGENTS_MD_FILENAME.to_string()]);

        assert!(result.is_some());
        let content = result.unwrap();
        assert!(content.contains("Test content"));
        assert!(content.contains("### Directory-Specific Hints:"));
    });
}

#[test]
fn test_load_hints_from_directory_no_file() {
    temp_env::with_var(DYNAMIC_SUBDIRECTORY_HINT_LOADING_ENV, Some("true"), || {
        let temp_dir = tempfile::tempdir().unwrap();
        let subdir = temp_dir.path().join("subdir");
        std::fs::create_dir(&subdir).unwrap();
        let result =
            load_hints_from_directory(&subdir, temp_dir.path(), &[AGENTS_MD_FILENAME.to_string()]);

        assert!(result.is_none());
    });
}

#[test]
fn test_load_hints_from_directory_with_imports() {
    temp_env::with_var(DYNAMIC_SUBDIRECTORY_HINT_LOADING_ENV, Some("true"), || {
        let temp_dir = tempfile::tempdir().unwrap();
        let subdir = temp_dir.path().join("subdir");
        std::fs::create_dir(&subdir).unwrap();

        // Create an included file in root (to verify we can import from parent)
        let included_path = temp_dir.path().join("included.md");
        std::fs::write(&included_path, "Included content").unwrap();

        let agents_path = subdir.join("AGENTS.md");
        std::fs::write(&agents_path, "Main content\n@../included.md\n").unwrap();

        let result =
            load_hints_from_directory(&subdir, temp_dir.path(), &[AGENTS_MD_FILENAME.to_string()]);

        assert!(result.is_some());
        let content = result.unwrap();
        assert!(content.contains("Main content"));
        assert!(content.contains("Included content"));
    });
}
