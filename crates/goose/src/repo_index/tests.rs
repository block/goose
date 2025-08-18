#[cfg(test)]
#[cfg(feature = "repo-index")]
mod repo_index_tests {
    use super::super::*;
    use std::io::Cursor;
    use std::path::Path;
    use tempfile::tempdir;

    #[test]
    fn extracts_rust_function_and_doc() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("lib.rs");
        std::fs::write(&file_path, "/// Adds two numbers\nfn add(a: i32, b: i32) -> i32 { a + b }\n").unwrap();
        let mut buf: Vec<u8> = Vec::new();
        let opts = crate::repo_index::RepoIndexOptions::builder()
            .root(dir.path())
            .output_writer(&mut buf)
            .build();
        let stats = crate::repo_index::index_repository(opts).unwrap();
        assert_eq!(stats.files_indexed, 1);
        let out = String::from_utf8(buf).unwrap();
        assert!(out.contains("add"), "output: {out}");
        assert!(out.contains("Adds two numbers"), "output: {out}");
        assert!(stats.entities_indexed >= 1);
    }
}
