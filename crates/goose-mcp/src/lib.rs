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
pub mod develop;
pub mod developer;
pub mod mcp_server_runner;
mod memory;
pub mod tutorial;

pub use autovisualiser::AutoVisualiserRouter;
pub use computercontroller::ComputerControllerServer;
pub use develop::DevelopServer;
pub use developer::rmcp_developer::DeveloperServer;
pub use memory::MemoryServer;
pub use tutorial::TutorialServer;

pub type SpawnServerFn = fn(tokio::io::DuplexStream, tokio::io::DuplexStream);

pub struct BuiltinDef {
    pub name: &'static str,
    pub spawn_server: SpawnServerFn,
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
        (
            stringify!($name),
            BuiltinDef {
                name: stringify!($name),
                spawn_server: spawn,
            },
        )
    }};
}

pub static BUILTIN_EXTENSIONS: Lazy<HashMap<&'static str, BuiltinDef>> = Lazy::new(|| {
    HashMap::from([
        builtin!(developer, DeveloperServer),
        builtin!(autovisualiser, AutoVisualiserRouter),
        builtin!(computercontroller, ComputerControllerServer),
        builtin!(memory, MemoryServer),
        builtin!(tutorial, TutorialServer),
    ])
});
