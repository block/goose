pub mod ascii_art;
pub mod file_completion;
pub mod json;
pub mod layout;
pub mod message_format;
pub mod sanitize;
pub mod spinner;
pub mod styles;
pub mod syntax;
pub mod termimad_renderer;

/// Default context limit when model config is unavailable
pub const DEFAULT_CONTEXT_LIMIT: usize = 128_000;
