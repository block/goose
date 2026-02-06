use etcetera::AppStrategyArgs;
use once_cell::sync::Lazy;
use rmcp::{ServerHandler, ServiceExt};
use std::collections::HashMap;

pub static APP_STRATEGY: Lazy<AppStrategyArgs> = Lazy::new(|| AppStrategyArgs {
    top_level_domain: "Block".to_string(),
    author: "Block".to_string(),
    app_name: "goose".to_string(),
});

pub mod autovisualiser;
pub mod computercontroller;
pub mod developer;
pub mod mcp_server_runner;
mod memory;
pub mod tutorial;

pub use autovisualiser::AutoVisualiserRouter;
pub use computercontroller::ComputerControllerServer;
pub use developer::rmcp_developer::DeveloperServer;
pub use memory::MemoryServer;
pub use tutorial::TutorialServer;

/// Type definition for a function that spawns and serves a builtin extension server
pub type SpawnServerFn = fn(tokio::io::DuplexStream, tokio::io::DuplexStream);

/// Definition of a builtin extension with metadata for config migration
pub struct BuiltinExtensionDef {
    pub name: &'static str,
    pub display_name: &'static str,
    pub description: &'static str,
    pub default_enabled: bool,
    pub spawn_fn: SpawnServerFn,
}

fn spawn_and_serve<S>(
    name: &'static str,
    server: S,
    transport: (tokio::io::DuplexStream, tokio::io::DuplexStream),
) where
    S: ServerHandler + Send + 'static,
{
    tokio::spawn(async move {
        match server.serve(transport).await {
            Ok(running) => {
                let _ = running.waiting().await;
            }
            Err(e) => tracing::error!(builtin = name, error = %e, "server error"),
        }
    });
}

macro_rules! builtin {
    ($name:ident, $server_ty:ty) => {{
        fn spawn(r: tokio::io::DuplexStream, w: tokio::io::DuplexStream) {
            spawn_and_serve(stringify!($name), <$server_ty>::new(), (r, w));
        }
        (stringify!($name), spawn as SpawnServerFn)
    }};
}

pub static BUILTIN_EXTENSIONS: Lazy<HashMap<&'static str, SpawnServerFn>> = Lazy::new(|| {
    HashMap::from([
        builtin!(developer, DeveloperServer),
        builtin!(autovisualiser, AutoVisualiserRouter),
        builtin!(computercontroller, ComputerControllerServer),
        builtin!(memory, MemoryServer),
        builtin!(tutorial, TutorialServer),
    ])
});

/// Builtin extension definitions with metadata for config migration
pub static BUILTIN_EXTENSION_DEFS: Lazy<HashMap<&'static str, BuiltinExtensionDef>> = Lazy::new(
    || {
        let mut map = HashMap::new();

        map.insert(
            "developer",
            BuiltinExtensionDef {
                name: "developer",
                display_name: "Developer",
                description: "General development tools useful for software engineering.",
                default_enabled: true,
                spawn_fn: BUILTIN_EXTENSIONS.get("developer").copied().unwrap(),
            },
        );

        map.insert(
            "computercontroller",
            BuiltinExtensionDef {
                name: "computercontroller",
                display_name: "Computer Controller",
                description: "General computer control tools that don't require you to be a developer or engineer.",
                default_enabled: false,
                spawn_fn: BUILTIN_EXTENSIONS.get("computercontroller").copied().unwrap(),
            },
        );

        map.insert(
            "autovisualiser",
            BuiltinExtensionDef {
                name: "autovisualiser",
                display_name: "Auto Visualiser",
                description: "Data visualization and UI generation tools.",
                default_enabled: false,
                spawn_fn: BUILTIN_EXTENSIONS.get("autovisualiser").copied().unwrap(),
            },
        );

        map.insert(
            "memory",
            BuiltinExtensionDef {
                name: "memory",
                display_name: "Memory",
                description: "Teach goose your preferences as you go.",
                default_enabled: false,
                spawn_fn: BUILTIN_EXTENSIONS.get("memory").copied().unwrap(),
            },
        );

        map.insert(
            "tutorial",
            BuiltinExtensionDef {
                name: "tutorial",
                display_name: "Tutorial",
                description: "Access interactive tutorials and guides.",
                default_enabled: false,
                spawn_fn: BUILTIN_EXTENSIONS.get("tutorial").copied().unwrap(),
            },
        );

        map
    },
);
