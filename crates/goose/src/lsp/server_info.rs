use anyhow::{anyhow, Result};
use once_cell::sync::Lazy;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct LspServerInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub language_ids: Vec<&'static str>,
    pub extensions: Vec<&'static str>,
    pub root_patterns: Vec<&'static str>,
    pub command_fn: fn() -> Result<(String, Vec<String>)>,
}

fn typescript_command() -> Result<(String, Vec<String>)> {
    if which::which("typescript-language-server").is_ok() {
        return Ok((
            "typescript-language-server".to_string(),
            vec!["--stdio".to_string()],
        ));
    }

    if which::which("npx").is_ok() {
        return Ok((
            "npx".to_string(),
            vec![
                "-y".to_string(),
                "typescript-language-server".to_string(),
                "--stdio".to_string(),
            ],
        ));
    }

    Err(anyhow!(
        "TypeScript language server not found. Install with: npm install -g typescript-language-server typescript"
    ))
}

fn python_command() -> Result<(String, Vec<String>)> {
    if which::which("pyright-langserver").is_ok() {
        return Ok((
            "pyright-langserver".to_string(),
            vec!["--stdio".to_string()],
        ));
    }

    if which::which("pylsp").is_ok() {
        return Ok(("pylsp".to_string(), vec![]));
    }

    Err(anyhow!(
        "Python language server not found. Install with: pip install pyright or pip install python-lsp-server"
    ))
}

fn rust_command() -> Result<(String, Vec<String>)> {
    if which::which("rust-analyzer").is_ok() {
        return Ok(("rust-analyzer".to_string(), vec![]));
    }

    Err(anyhow!(
        "Rust language server not found. Install with: rustup component add rust-analyzer"
    ))
}

fn go_command() -> Result<(String, Vec<String>)> {
    if which::which("gopls").is_ok() {
        return Ok(("gopls".to_string(), vec![]));
    }

    Err(anyhow!(
        "Go language server not found. Install with: go install golang.org/x/tools/gopls@latest"
    ))
}

fn java_command() -> Result<(String, Vec<String>)> {
    if which::which("jdtls").is_ok() {
        return Ok(("jdtls".to_string(), vec![]));
    }

    Err(anyhow!(
        "Java language server not found. Download from: https://download.eclipse.org/jdtls/snapshots/"
    ))
}

pub static BUILTIN_LSP_SERVERS: Lazy<HashMap<&'static str, LspServerInfo>> = Lazy::new(|| {
    let mut servers = HashMap::new();

    servers.insert(
        "typescript",
        LspServerInfo {
            id: "typescript",
            name: "TypeScript Language Server",
            language_ids: vec![
                "typescript",
                "javascript",
                "typescriptreact",
                "javascriptreact",
            ],
            extensions: vec![".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs"],
            root_patterns: vec!["package.json", "tsconfig.json", "jsconfig.json"],
            command_fn: typescript_command,
        },
    );

    servers.insert(
        "python",
        LspServerInfo {
            id: "python",
            name: "Python Language Server",
            language_ids: vec!["python"],
            extensions: vec![".py", ".pyi"],
            root_patterns: vec!["pyproject.toml", "setup.py", "requirements.txt", ".git"],
            command_fn: python_command,
        },
    );

    servers.insert(
        "rust",
        LspServerInfo {
            id: "rust",
            name: "Rust Analyzer",
            language_ids: vec!["rust"],
            extensions: vec![".rs"],
            root_patterns: vec!["Cargo.toml", "Cargo.lock"],
            command_fn: rust_command,
        },
    );

    servers.insert(
        "go",
        LspServerInfo {
            id: "go",
            name: "Go Language Server",
            language_ids: vec!["go"],
            extensions: vec![".go"],
            root_patterns: vec!["go.mod", "go.sum"],
            command_fn: go_command,
        },
    );

    servers.insert(
        "java",
        LspServerInfo {
            id: "java",
            name: "Eclipse JDT Language Server",
            language_ids: vec!["java"],
            extensions: vec![".java"],
            root_patterns: vec!["pom.xml", "build.gradle", ".git"],
            command_fn: java_command,
        },
    );

    servers
});

pub fn get_lsp_server_info(id: &str) -> Option<&'static LspServerInfo> {
    BUILTIN_LSP_SERVERS.get(id)
}

pub fn get_server_by_extension(ext: &str) -> Option<&'static LspServerInfo> {
    BUILTIN_LSP_SERVERS
        .values()
        .find(|server| server.extensions.contains(&ext))
}
