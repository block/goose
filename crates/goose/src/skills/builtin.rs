use include_dir::{Dir, include_dir};

static BUILTIN_SKILLS_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/src/skills/builtins");

pub fn get_all() -> Vec<&'static str> {
    BUILTIN_SKILLS_DIR
        .files()
        .filter(|f| f.path().extension().is_some_and(|ext| ext == "md"))
        .filter_map(|f| f.contents_utf8())
        .collect()
}
