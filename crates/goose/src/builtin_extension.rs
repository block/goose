use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::RwLock;

/// Type definition for a function that spawns and serves a builtin extension server
pub type SpawnServerFn = fn(tokio::io::DuplexStream, tokio::io::DuplexStream);

/// Definition of a builtin extension
#[derive(Clone, Copy)]
pub struct BuiltinDef {
    pub name: &'static str,
    pub spawn_server: SpawnServerFn,
}

/// Global registry of builtin extensions
static BUILTIN_REGISTRY: Lazy<RwLock<HashMap<&'static str, BuiltinDef>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

/// Register a builtin extension into the global registry
pub fn register_builtin_extension(name: &'static str, def: BuiltinDef) {
    BUILTIN_REGISTRY.write().unwrap().insert(name, def);
}

pub fn register_builtin_extensions(extensions: HashMap<&'static str, BuiltinDef>) {
    let mut registry = BUILTIN_REGISTRY.write().unwrap();
    registry.extend(extensions);
}

/// Get a copy of all registered builtin extensions
pub fn get_builtin_extensions() -> HashMap<&'static str, BuiltinDef> {
    BUILTIN_REGISTRY.read().unwrap().clone()
}
