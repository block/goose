pub mod analyze;
mod editor_models;
mod lang;
pub mod paths;
mod shell;
mod text_editor;

pub use text_editor::{FileDiff, WriteMode, FILE_DIFF_MIME_TYPE};

pub mod rmcp_developer;

#[cfg(test)]
mod tests;
