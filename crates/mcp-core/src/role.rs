// passthrough, which will be deleted with the rest of the mcp-core crate after it is no longer used
// needed because it has internal references in this crate which leak out to usages used in goose etc crates
pub use rmcp::model::Role;
