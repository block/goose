use std::path::Path;

/// Get the markdown language identifier for a file extension
pub fn get_language_identifier(path: &Path) -> &'static str {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("rs") => "rust",
        Some("hs") => "haskell",
        Some("rkt") | Some("scm") => "scheme",
        Some("py") => "python",
        Some("js") => "javascript",
        Some("ts") => "typescript",
        Some("json") => "json",
        Some("toml") => "toml",
        Some("yaml") | Some("yml") => "yaml",
        Some("sh") => "bash",
        Some("ps1") => "powershell",
        Some("bat") | Some("cmd") => "batch",
        Some("vbs") => "vbscript",
        Some("go") => "go",
        Some("md") => "markdown",
        Some("html") => "html",
        Some("css") => "css",
        Some("sql") => "sql",
        Some("java") => "java",
        Some("cpp") | Some("cc") | Some("cxx") => "cpp",
        Some("c") => "c",
        Some("h") | Some("hpp") => "cpp",
        Some("rb") => "ruby",
        Some("php") => "php",
        Some("swift") => "swift",
        Some("kt") | Some("kts") => "kotlin",
        Some("scala") => "scala",
        Some("r") => "r",
        Some("m") => "matlab",
        Some("pl") => "perl",
        Some("dockerfile") => "dockerfile",
        _ => "",
    }
}
