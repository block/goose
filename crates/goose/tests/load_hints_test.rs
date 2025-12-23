use goose::hints::{
    load_hints_from_directory, AGENTS_MD_FILENAME, DYNAMIC_SUBDIRECTORY_HINT_LOADING_ENV,
};

#[test]
fn test_load_hints_from_directory_env_gating_and_basic_load() {
    let cases: &[(Option<&str>, bool)] = &[
        (None, true),           // default: enabled
        (Some("true"), true),   // explicit enable
        (Some("false"), false), // explicit disable
        (Some("0"), false),     // explicit disable (0)
    ];

    for (env_value, should_load) in cases {
        temp_env::with_var(DYNAMIC_SUBDIRECTORY_HINT_LOADING_ENV, *env_value, || {
            let temp_dir = tempfile::tempdir().unwrap();
            let subdir = temp_dir.path().join("subdir");
            std::fs::create_dir(&subdir).unwrap();
            std::fs::write(subdir.join(AGENTS_MD_FILENAME), "Test content").unwrap();

            let result = load_hints_from_directory(
                &subdir,
                temp_dir.path(),
                &[AGENTS_MD_FILENAME.to_string()],
            );

            assert_eq!(
                result.is_some(),
                *should_load,
                "env={:?} should_load={}",
                env_value,
                should_load
            );
            if *should_load {
                let content = result.unwrap();
                assert!(content.contains("Test content"));
                assert!(content.contains("### Directory-Specific Hints:"));
            }
        });
    }
}

#[test]
fn test_load_hints_from_directory_no_files_returns_none() {
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
fn test_load_hints_from_directory_expands_imports() {
    temp_env::with_var(DYNAMIC_SUBDIRECTORY_HINT_LOADING_ENV, Some("true"), || {
        let temp_dir = tempfile::tempdir().unwrap();
        let subdir = temp_dir.path().join("subdir");
        std::fs::create_dir(&subdir).unwrap();

        // Create an included file in root (to verify we can import from parent)
        let included_path = temp_dir.path().join("included.md");
        std::fs::write(&included_path, "Included content").unwrap();

        let agents_path = subdir.join(AGENTS_MD_FILENAME);
        std::fs::write(&agents_path, "Main content\n@../included.md\n").unwrap();

        let result =
            load_hints_from_directory(&subdir, temp_dir.path(), &[AGENTS_MD_FILENAME.to_string()]);

        assert!(result.is_some());
        let content = result.unwrap();
        assert!(content.contains("Main content"));
        assert!(content.contains("Included content"));
    });
}
