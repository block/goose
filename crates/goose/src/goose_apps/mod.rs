//! goose Apps module
//!
//! This module contains types and utilities for working with goose Apps,
//! which are UI resources that can be rendered in an MCP server or native
//! goose apps, or something in between.

pub mod resource;

pub use resource::{CspMetadata, McpAppResource, ResourceMetadata, UiMetadata};
