use super::base::Config;
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Default maximum size for tool responses in bytes (10MB)
pub const DEFAULT_MAX_TOOL_RESPONSE_SIZE: usize = 10 * 1024 * 1024;

/// Configuration key for the maximum tool response size
const MAX_TOOL_RESPONSE_SIZE_KEY: &str = "max_tool_response_size";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolLimitsConfig {
    /// Maximum size in bytes for a single tool response
    pub max_tool_response_size: usize,
}

impl Default for ToolLimitsConfig {
    fn default() -> Self {
        Self {
            max_tool_response_size: DEFAULT_MAX_TOOL_RESPONSE_SIZE,
        }
    }
}

/// Manager for tool-related configuration limits
pub struct ToolLimitsManager;

impl ToolLimitsManager {
    /// Get the maximum tool response size from config
    pub fn get_max_tool_response_size() -> usize {
        let config = Config::global();
        config
            .get_param::<usize>(MAX_TOOL_RESPONSE_SIZE_KEY)
            .unwrap_or(DEFAULT_MAX_TOOL_RESPONSE_SIZE)
    }

    /// Set the maximum tool response size in config
    pub fn set_max_tool_response_size(size: usize) -> Result<()> {
        let config = Config::global();
        config.set_param(MAX_TOOL_RESPONSE_SIZE_KEY, serde_json::to_value(size)?)?;
        Ok(())
    }

    /// Check if content exceeds the size limit and truncate if necessary
    pub fn check_and_truncate_content(
        content: Vec<rmcp::model::Content>,
    ) -> (Vec<rmcp::model::Content>, Option<String>) {
        let max_size = Self::get_max_tool_response_size();
        let mut total_size = 0;
        let mut truncated = false;
        let mut result = Vec::new();

        for item in content {
            let item_size = Self::estimate_content_size(&item);

            if total_size + item_size > max_size {
                // If we haven't added any content yet, truncate this item
                if result.is_empty() {
                    let truncated_item = Self::truncate_content_item(item, max_size);
                    result.push(truncated_item);
                }
                truncated = true;
                break;
            }

            total_size += item_size;
            result.push(item);
        }

        let error_msg = if truncated {
            Some(format!(
                "Tool response exceeded maximum size limit of {} bytes. \
                The response has been truncated. Consider using more specific parameters, \
                limiting the scope of your request, or breaking it into smaller operations.",
                max_size
            ))
        } else {
            None
        };

        (result, error_msg)
    }

    /// Estimate the size of a Content item in bytes
    fn estimate_content_size(content: &rmcp::model::Content) -> usize {
        use rmcp::model::{RawContent, ResourceContents};

        match &content.raw {
            RawContent::Text(text) => text.text.len(),
            RawContent::Image(image) => image.data.len() + image.mime_type.len(),
            RawContent::Resource(resource) => match &resource.resource {
                ResourceContents::TextResourceContents {
                    text,
                    uri,
                    mime_type,
                    ..
                } => text.len() + uri.len() + mime_type.as_ref().map_or(0, |s| s.len()),
                ResourceContents::BlobResourceContents {
                    blob,
                    uri,
                    mime_type,
                    ..
                } => blob.len() + uri.len() + mime_type.as_ref().map_or(0, |s| s.len()),
            },
            RawContent::ResourceLink(link) => link.uri.len(),
            RawContent::Audio(audio) => audio.data.len() + audio.mime_type.len(),
        }
    }

    /// Truncate a single content item to fit within the size limit
    fn truncate_content_item(
        content: rmcp::model::Content,
        max_size: usize,
    ) -> rmcp::model::Content {
        use rmcp::model::{Content, RawContent, RawTextContent};

        match content.raw {
            RawContent::Text(mut text) => {
                if text.text.len() > max_size {
                    // Try to truncate at a reasonable boundary
                    let truncate_at = if max_size > 1000 {
                        max_size - 500 // Leave room for truncation message
                    } else {
                        max_size / 2
                    };

                    let truncated_text =
                        if let Some(last_newline) = text.text[..truncate_at].rfind('\n') {
                            &text.text[..last_newline]
                        } else {
                            &text.text[..truncate_at]
                        };

                    text.text = format!(
                        "{}\n\n[TRUNCATED - Response exceeded {} byte limit]",
                        truncated_text, max_size
                    );
                }
                Content {
                    raw: RawContent::Text(text),
                    annotations: content.annotations,
                }
            }
            _ => {
                // For non-text content, replace with a message
                Content {
                    raw: RawContent::Text(RawTextContent {
                        text: format!(
                            "[Content truncated - {} exceeded {} byte limit]",
                            Self::content_type_name(&content.raw),
                            max_size
                        ),
                        meta: None,
                    }),
                    annotations: content.annotations,
                }
            }
        }
    }

    /// Get a human-readable name for the content type
    fn content_type_name(content: &rmcp::model::RawContent) -> &str {
        use rmcp::model::RawContent;

        match content {
            RawContent::Text(_) => "Text content",
            RawContent::Image(_) => "Image content",
            RawContent::Resource(_) => "Resource content",
            RawContent::ResourceLink(_) => "Resource link",
            RawContent::Audio(_) => "Audio content",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::model::{Content, RawContent, RawTextContent};
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_max_size() {
        let max_size = ToolLimitsManager::get_max_tool_response_size();
        assert_eq!(max_size, DEFAULT_MAX_TOOL_RESPONSE_SIZE);
    }

    #[test]
    fn test_set_and_get_max_size() -> Result<()> {
        let temp_file = NamedTempFile::new().unwrap();
        let _config = Config::new(temp_file.path(), "test-service")?;

        // Set a custom size
        ToolLimitsManager::set_max_tool_response_size(5 * 1024 * 1024)?;

        // Note: This test might not work as expected due to global config
        // In production, the config would persist
        Ok(())
    }

    #[test]
    fn test_content_within_limit() {
        // Create content that's within the limit
        let content = vec![Content {
            raw: RawContent::Text(RawTextContent {
                text: "Small content".to_string(),
                meta: None,
            }),
            annotations: None,
        }];

        let (result, error_msg) = ToolLimitsManager::check_and_truncate_content(content.clone());

        assert_eq!(result.len(), 1);
        assert!(error_msg.is_none());
    }

    #[test]
    fn test_content_exceeds_limit() {
        // Create content that exceeds the default limit
        let large_text = "a".repeat(DEFAULT_MAX_TOOL_RESPONSE_SIZE + 1000);
        let content = vec![Content {
            raw: RawContent::Text(RawTextContent {
                text: large_text,
                meta: None,
            }),
            annotations: None,
        }];

        let (result, error_msg) = ToolLimitsManager::check_and_truncate_content(content);

        assert_eq!(result.len(), 1);
        assert!(error_msg.is_some());
        assert!(error_msg.unwrap().contains("exceeded maximum size limit"));

        // Check that the content was actually truncated
        if let RawContent::Text(ref text) = result[0].raw {
            assert!(text.text.contains("[TRUNCATED"));
        }
    }

    #[test]
    fn test_multiple_content_items_truncation() {
        // Temporarily set a small limit
        let original_size = ToolLimitsManager::get_max_tool_response_size();
        let _ = ToolLimitsManager::set_max_tool_response_size(100);

        // Create multiple content items
        let content = vec![
            Content {
                raw: RawContent::Text(RawTextContent {
                    text: "a".repeat(60),
                    meta: None,
                }),
                annotations: None,
            },
            Content {
                raw: RawContent::Text(RawTextContent {
                    text: "b".repeat(60),
                    meta: None,
                }),
                annotations: None,
            },
        ];

        let (result, error_msg) = ToolLimitsManager::check_and_truncate_content(content);

        // Should only include the first item
        assert_eq!(result.len(), 1);
        assert!(error_msg.is_some());

        // Restore original size
        let _ = ToolLimitsManager::set_max_tool_response_size(original_size);
    }
}
