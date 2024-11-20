use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct TextContent {
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct ImageContent {
    pub data: String,
    pub mime_type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all="camelCase")]
/// Content passed to or from an LLM
pub enum Content {
    Text(TextContent),
    Image(ImageContent),
}

impl Content {
    pub fn text<S: Into<String>>(text: S) -> Self {
        Content::Text(TextContent { text: text.into() })
    }

    pub fn image<S: Into<String>, T: Into<String>>(data: S, mime_type: T) -> Self {
        Content::Image(ImageContent {
            data: data.into(),
            mime_type: mime_type.into(),
        })
    }

    /// Get the text content if this is a TextContent variant
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Content::Text(text) => Some(&text.text),
            _ => None,
        }
    }

    /// Get the image content if this is an ImageContent variant
    pub fn as_image(&self) -> Option<(&str, &str)> {
        match self {
            Content::Image(image) => Some((&image.data, &image.mime_type)),
            _ => None,
        }
    }
}
