pub mod ascii_art;
pub mod json;
pub mod layout;
pub mod message_format;
pub mod sanitize;
pub mod styles;
pub mod termimad_renderer;

/// Default context limit when model config is unavailable
pub const DEFAULT_CONTEXT_LIMIT: usize = 128_000;
