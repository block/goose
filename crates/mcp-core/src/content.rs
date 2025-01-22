/// Content sent around agents, extensions, and LLMs
/// The various content types can be display to humans but also understood by models
/// They include optional annotations used to help inform agent usage
use super::role::Role;
use crate::resource::ResourceContents;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Annotations {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audience: Option<Vec<Role>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<DateTime<Utc>>,
}

impl Annotations {
    /// Creates a new Annotations instance specifically for resources
    /// optional priority, and a timestamp (defaults to now if None)
    pub fn for_resource(priority: f32, timestamp: DateTime<Utc>) -> Self {
        assert!(
            (0.0..=1.0).contains(&priority),
            "Priority {priority} must be between 0.0 and 1.0"
        );
        Annotations {
            priority: Some(priority),
            timestamp: Some(timestamp),
            audience: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextContent {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageContent {
    pub data: String,
    pub mime_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmbeddedResource {
    pub resource: ResourceContents,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,
}

impl EmbeddedResource {
    pub fn get_text(&self) -> String {
        match &self.resource {
            ResourceContents::TextResourceContents { text, .. } => text.clone(),
            _ => String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Content {
    Text(TextContent),
    Image(ImageContent),
    Resource(EmbeddedResource),
}

impl Content {
    pub fn text<S: Into<String>>(text: S) -> Self {
        Content::Text(TextContent {
            text: text.into(),
            annotations: None,
        })
    }

    pub fn image<S: Into<String>, T: Into<String>>(data: S, mime_type: T) -> Self {
        Content::Image(ImageContent {
            data: data.into(),
            mime_type: mime_type.into(),
            annotations: None,
        })
    }

    pub fn resource(resource: ResourceContents) -> Self {
        Content::Resource(EmbeddedResource {
            resource,
            annotations: None,
        })
    }

    pub fn embedded_text<S: Into<String>, T: Into<String>>(uri: S, content: T) -> Self {
        Content::Resource(EmbeddedResource {
            resource: ResourceContents::TextResourceContents {
                uri: uri.into(),
                mime_type: Some("text".to_string()),
                text: content.into(),
            },
            annotations: None,
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

    /// Set the audience for the content
    pub fn with_audience(mut self, audience: Vec<Role>) -> Self {
        let annotations = match &mut self {
            Content::Text(text) => &mut text.annotations,
            Content::Image(image) => &mut image.annotations,
            Content::Resource(resource) => &mut resource.annotations,
        };
        *annotations = Some(match annotations.take() {
            Some(mut a) => {
                a.audience = Some(audience);
                a
            }
            None => Annotations {
                audience: Some(audience),
                priority: None,
                timestamp: None,
            },
        });
        self
    }

    /// Set the priority for the content
    /// # Panics
    /// Panics if priority is not between 0.0 and 1.0 inclusive
    pub fn with_priority(mut self, priority: f32) -> Self {
        if !(0.0..=1.0).contains(&priority) {
            panic!("Priority must be between 0.0 and 1.0");
        }
        let annotations = match &mut self {
            Content::Text(text) => &mut text.annotations,
            Content::Image(image) => &mut image.annotations,
            Content::Resource(resource) => &mut resource.annotations,
        };
        *annotations = Some(match annotations.take() {
            Some(mut a) => {
                a.priority = Some(priority);
                a
            }
            None => Annotations {
                audience: None,
                priority: Some(priority),
                timestamp: None,
            },
        });
        self
    }

    /// Get the audience if set
    pub fn audience(&self) -> Option<&Vec<Role>> {
        match self {
            Content::Text(text) => text.annotations.as_ref().and_then(|a| a.audience.as_ref()),
            Content::Image(image) => image.annotations.as_ref().and_then(|a| a.audience.as_ref()),
            Content::Resource(resource) => resource
                .annotations
                .as_ref()
                .and_then(|a| a.audience.as_ref()),
        }
    }

    /// Get the priority if set
    pub fn priority(&self) -> Option<f32> {
        match self {
            Content::Text(text) => text.annotations.as_ref().and_then(|a| a.priority),
            Content::Image(image) => image.annotations.as_ref().and_then(|a| a.priority),
            Content::Resource(resource) => resource.annotations.as_ref().and_then(|a| a.priority),
        }
    }

    pub fn unannotated(&self) -> Self {
        match self {
            Content::Text(text) => Content::text(text.text.clone()),
            Content::Image(image) => Content::image(image.data.clone(), image.mime_type.clone()),
            Content::Resource(resource) => Content::resource(resource.resource.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_text() {
        let content = Content::text("hello");
        assert_eq!(content.as_text(), Some("hello"));
        assert_eq!(content.as_image(), None);
    }

    #[test]
    fn test_content_image() {
        let content = Content::image("data", "image/png");
        assert_eq!(content.as_text(), None);
        assert_eq!(content.as_image(), Some(("data", "image/png")));
    }

    #[test]
    fn test_content_annotations_basic() {
        let content = Content::text("hello")
            .with_audience(vec![Role::User])
            .with_priority(0.5);
        assert_eq!(content.audience(), Some(&vec![Role::User]));
        assert_eq!(content.priority(), Some(0.5));
    }

    #[test]
    fn test_content_annotations_order_independence() {
        let content1 = Content::text("hello")
            .with_audience(vec![Role::User])
            .with_priority(0.5);
        let content2 = Content::text("hello")
            .with_priority(0.5)
            .with_audience(vec![Role::User]);

        assert_eq!(content1.audience(), content2.audience());
        assert_eq!(content1.priority(), content2.priority());
    }

    #[test]
    fn test_content_annotations_overwrite() {
        let content = Content::text("hello")
            .with_audience(vec![Role::User])
            .with_priority(0.5)
            .with_audience(vec![Role::Assistant])
            .with_priority(0.8);

        assert_eq!(content.audience(), Some(&vec![Role::Assistant]));
        assert_eq!(content.priority(), Some(0.8));
    }

    #[test]
    fn test_content_annotations_image() {
        let content = Content::image("data", "image/png")
            .with_audience(vec![Role::User])
            .with_priority(0.5);

        assert_eq!(content.audience(), Some(&vec![Role::User]));
        assert_eq!(content.priority(), Some(0.5));
    }

    #[test]
    fn test_content_annotations_preservation() {
        let text_content = Content::text("hello")
            .with_audience(vec![Role::User])
            .with_priority(0.5);

        match &text_content {
            Content::Text(TextContent { annotations, .. }) => {
                assert!(annotations.is_some());
                let ann = annotations.as_ref().unwrap();
                assert_eq!(ann.audience, Some(vec![Role::User]));
                assert_eq!(ann.priority, Some(0.5));
            }
            _ => panic!("Expected Text content"),
        }
    }

    #[test]
    #[should_panic(expected = "Priority must be between 0.0 and 1.0")]
    fn test_invalid_priority() {
        Content::text("hello").with_priority(1.5);
    }

    #[test]
    fn test_unannotated() {
        let content = Content::text("hello")
            .with_audience(vec![Role::User])
            .with_priority(0.5);
        let unannotated = content.unannotated();
        assert_eq!(unannotated.audience(), None);
        assert_eq!(unannotated.priority(), None);
    }

    #[test]
    fn test_partial_annotations() {
        let content = Content::text("hello").with_priority(0.5);
        assert_eq!(content.audience(), None);
        assert_eq!(content.priority(), Some(0.5));

        let content = Content::text("hello").with_audience(vec![Role::User]);
        assert_eq!(content.audience(), Some(&vec![Role::User]));
        assert_eq!(content.priority(), None);
    }
}
