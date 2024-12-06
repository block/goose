use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use anyhow::{Result, anyhow};
use url::Url;

/// Represents a resource in the system with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    /// URI representing the resource location (e.g., "file:///path/to/file" or "str:///content")
    pub uri: String,
    /// Name of the resource
    pub name: String,
    /// Last modified timestamp
    pub timestamp: DateTime<Utc>,
    /// Priority of the resource (higher number means higher priority)
    pub priority: i32,
    /// Optional description of the resource
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// MIME type of the resource content ("text" or "blob")
    #[serde(default = "default_mime_type")]
    pub mime_type: String,
}

fn default_mime_type() -> String {
    "text".to_string()
}

impl Resource {
    /// Creates a new Resource from a URI with explicit mime type
    pub fn new<S: AsRef<str>>(uri: S, mime_type: Option<String>, name: Option<String>) -> Result<Self> {
        let uri = uri.as_ref();
        let url = Url::parse(uri)
            .map_err(|e| anyhow!("Invalid URI: {}", e))?;

        // Extract name from the path component of the URI
        // Use provided name if available, otherwise extract from URI
        let name = match name {
            Some(n) => n,
            None => url.path_segments()
                .and_then(|segments| segments.last())
                .unwrap_or("unnamed")
                .to_string()
        };

        // Use provided mime_type or default
        let mime_type = match mime_type {
            Some(t) if t == "text" || t == "blob" => t,
            _ => default_mime_type(),
        };

        Ok(Self {
            uri: uri.to_string(),
            name,
            timestamp: Utc::now(),
            priority: 0,
            description: None,
            mime_type,
        })
    }

    /// Creates a new Resource with explicit URI, name, and priority
    pub fn with_uri<S: Into<String>>(uri: S, name: S, priority: i32, mime_type: Option<String>) -> Result<Self> {
        let uri_string = uri.into();
        Url::parse(&uri_string)
            .map_err(|e| anyhow!("Invalid URI: {}", e))?;

        // Use provided mime_type or default
        let mime_type = match mime_type {
            Some(t) if t == "text" || t == "blob" => t,
            _ => default_mime_type(),
        };

        Ok(Self {
            uri: uri_string,
            name: name.into(),
            timestamp: Utc::now(),
            priority,
            description: None,
            mime_type,
        })
    }

    /// Updates the resource's timestamp to the current time
    pub fn update_timestamp(&mut self) {
        self.timestamp = Utc::now();
    }

    /// Sets the priority of the resource and returns self for method chaining
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Returns the scheme of the URI
    pub fn scheme(&self) -> Result<String> {
        let url = Url::parse(&self.uri)?;
        Ok(url.scheme().to_string())
    }

    /// Sets the description of the resource
    pub fn with_description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the MIME type of the resource
    pub fn with_mime_type<S: Into<String>>(mut self, mime_type: S) -> Self {
        let mime_type = mime_type.into();
        match mime_type.as_str() {
            "text" | "blob" => self.mime_type = mime_type,
            _ => self.mime_type = default_mime_type(),
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[test]
    fn test_new_resource_with_file_uri() -> Result<()> {
        let mut temp_file = NamedTempFile::new()?;
        writeln!(temp_file, "test content")?;
        
        let uri = Url::from_file_path(temp_file.path())
            .map_err(|_| anyhow!("Invalid file path"))?
            .to_string();
        
        let resource = Resource::new(&uri, Some("text".to_string()), None)?;
        assert!(resource.uri.starts_with("file:///"));
        assert_eq!(resource.priority, 0);
        assert_eq!(resource.mime_type, "text");
        assert_eq!(resource.scheme()?, "file");
        
        Ok(())
    }

    #[test]
    fn test_resource_with_str_uri() -> Result<()> {
        let test_content = "Hello, world!";
        let uri = format!("str:///{}", test_content);
        let resource = Resource::with_uri(uri.clone(), "test.txt".to_string(), 5, Some("text".to_string()))?;
        
        assert_eq!(resource.uri, uri);
        assert_eq!(resource.name, "test.txt");
        assert_eq!(resource.priority, 5);
        assert_eq!(resource.mime_type, "text");
        assert_eq!(resource.scheme()?, "str");
        
        Ok(())
    }

    #[test]
    fn test_mime_type_validation() -> Result<()> {
        // Test valid mime types
        let resource = Resource::new("file:///test.txt", Some("text".to_string()), None)?;
        assert_eq!(resource.mime_type, "text");

        let resource = Resource::new("file:///test.bin", Some("blob".to_string()), None)?;
        assert_eq!(resource.mime_type, "blob");

        // Test invalid mime type defaults to "text"
        let resource = Resource::new("file:///test.txt", Some("invalid".to_string()), None)?;
        assert_eq!(resource.mime_type, "text");

        // Test None defaults to "text"
        let resource = Resource::new("file:///test.txt", None, None)?;
        assert_eq!(resource.mime_type, "text");

        Ok(())
    }

    #[test]
    fn test_with_description() -> Result<()> {
        let resource = Resource::with_uri("file:///test.txt", "test.txt", 0, None)?
            .with_description("A test resource");
        
        assert_eq!(resource.description, Some("A test resource".to_string()));
        Ok(())
    }

    #[test]
    fn test_with_mime_type() -> Result<()> {
        let resource = Resource::with_uri("file:///test.txt", "test.txt", 0, None)?
            .with_mime_type("blob");
        
        assert_eq!(resource.mime_type, "blob");

        // Test invalid mime type defaults to "text"
        let resource = resource.with_mime_type("invalid");
        assert_eq!(resource.mime_type, "text");
        Ok(())
    }

    #[test]
    fn test_invalid_uri() {
        let result = Resource::new("not-a-uri", None, None);
        assert!(result.is_err());
    }
}