/// Type definition for a function that spawns and serves a builtin extension server
pub type SpawnServerFn = fn(tokio::io::DuplexStream, tokio::io::DuplexStream);

/// Definition of a builtin extension
#[derive(Clone, Copy)]
pub struct BuiltinDef {
    pub name: &'static str,
    pub spawn_server: SpawnServerFn,
}
