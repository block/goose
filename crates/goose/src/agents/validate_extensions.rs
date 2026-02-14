use crate::agents::ExtensionConfig;
use anyhow::Result;
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct BundledExtensionEntry {
    id: String,
    name: String,
    #[serde(rename = "type")]
    extension_type: String,
    #[allow(dead_code)]
    #[serde(default)]
    enabled: bool,
}

#[derive(Debug)]
pub struct ValidationError {
    pub index: usize,
    pub id: String,
    pub name: String,
    pub error: String,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}] {} (id={}): {}",
            self.index, self.name, self.id, self.error
        )
    }
}

#[derive(Debug)]
pub struct ValidationResult {
    pub total: usize,
    pub errors: Vec<ValidationError>,
}

impl ValidationResult {
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }
}

pub fn validate_bundled_extensions(path: &Path) -> Result<ValidationResult> {
    let content = std::fs::read_to_string(path)?;
    let raw_entries: Vec<serde_json::Value> = serde_json::from_str(&content)?;
    let total = raw_entries.len();
    let mut errors = Vec::new();

    for (index, entry) in raw_entries.iter().enumerate() {
        let meta: BundledExtensionEntry = match serde_json::from_value(entry.clone()) {
            Ok(m) => m,
            Err(e) => {
                errors.push(ValidationError {
                    index,
                    id: entry
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    name: entry
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    error: format!("missing required metadata fields: {e}"),
                });
                continue;
            }
        };

        if !ExtensionConfig::VALID_TYPES.contains(&meta.extension_type.as_str()) {
            errors.push(ValidationError {
                index,
                id: meta.id,
                name: meta.name,
                error: format!(
                    "unknown type \"{}\", expected one of: {}",
                    meta.extension_type,
                    ExtensionConfig::VALID_TYPES.join(", ")
                ),
            });
            continue;
        }

        let mut has_type_error = false;
        match meta.extension_type.as_str() {
            "streamable_http" => {
                if entry.get("url").is_some() && entry.get("uri").is_none() {
                    errors.push(ValidationError {
                        index,
                        id: meta.id.clone(),
                        name: meta.name.clone(),
                        error: "has \"url\" field but streamable_http expects \"uri\" — did you mean \"uri\"?".to_string(),
                    });
                    has_type_error = true;
                }
            }
            "stdio" => {
                if entry.get("cmd").is_none() {
                    errors.push(ValidationError {
                        index,
                        id: meta.id.clone(),
                        name: meta.name.clone(),
                        error: "stdio extension is missing required \"cmd\" field".to_string(),
                    });
                    has_type_error = true;
                }
            }
            _ => {}
        }

        if !has_type_error {
            if let Err(e) = serde_json::from_value::<ExtensionConfig>(entry.clone()) {
                errors.push(ValidationError {
                    index,
                    id: meta.id,
                    name: meta.name,
                    error: format!("failed to deserialize as ExtensionConfig: {e}"),
                });
            }
        }
    }

    Ok(ValidationResult { total, errors })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn write_json(content: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f
    }

    #[test]
    fn test_valid_builtin() {
        let f = write_json(
            r#"[{
            "id": "developer",
            "name": "developer",
            "display_name": "Developer",
            "description": "Dev tools",
            "enabled": true,
            "type": "builtin",
            "timeout": 300,
            "bundled": true
        }]"#,
        );
        let result = validate_bundled_extensions(f.path()).unwrap();
        assert!(result.is_ok());
        assert_eq!(result.total, 1);
    }

    #[test]
    fn test_valid_stdio() {
        let f = write_json(
            r#"[{
            "id": "googledrive",
            "name": "Google Drive",
            "description": "Google Drive integration",
            "enabled": false,
            "type": "stdio",
            "cmd": "uvx",
            "args": ["mcp_gdrive@latest"],
            "env_keys": [],
            "timeout": 300,
            "bundled": true
        }]"#,
        );
        let result = validate_bundled_extensions(f.path()).unwrap();
        assert!(result.is_ok());
    }

    #[test]
    fn test_valid_streamable_http() {
        let f = write_json(
            r#"[{
            "id": "asana",
            "name": "Asana",
            "display_name": "Asana",
            "description": "Manage Asana tasks",
            "enabled": false,
            "type": "streamable_http",
            "uri": "https://mcp.asana.com/mcp",
            "env_keys": [],
            "timeout": 300,
            "bundled": true
        }]"#,
        );
        let result = validate_bundled_extensions(f.path()).unwrap();
        assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_type_http() {
        let f = write_json(
            r#"[{
            "id": "asana",
            "name": "Asana",
            "description": "Manage Asana tasks",
            "enabled": false,
            "type": "http",
            "uri": "https://mcp.asana.com/mcp",
            "timeout": 300,
            "bundled": true
        }]"#,
        );
        let result = validate_bundled_extensions(f.path()).unwrap();
        assert!(!result.is_ok());
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].error.contains("unknown type \"http\""));
    }

    #[test]
    fn test_url_instead_of_uri() {
        let f = write_json(
            r#"[{
            "id": "neighborhood",
            "name": "Neighborhood",
            "description": "Neighborhood tools",
            "enabled": false,
            "type": "streamable_http",
            "url": "https://example.com/mcp",
            "timeout": 300,
            "bundled": true
        }]"#,
        );
        let result = validate_bundled_extensions(f.path()).unwrap();
        assert!(!result.is_ok());
        assert!(result.errors.iter().any(|e| e.error.contains("uri")));
    }

    #[test]
    fn test_missing_cmd_for_stdio() {
        let f = write_json(
            r#"[{
            "id": "test",
            "name": "Test",
            "description": "Test extension",
            "enabled": false,
            "type": "stdio",
            "args": [],
            "timeout": 300,
            "bundled": true
        }]"#,
        );
        let result = validate_bundled_extensions(f.path()).unwrap();
        assert!(!result.is_ok());
        assert!(result.errors[0].error.contains("cmd"));
    }

    #[test]
    fn test_valid_entries_before_invalid_still_pass() {
        let f = write_json(
            r#"[
            {
                "id": "developer",
                "name": "developer",
                "description": "Dev tools",
                "enabled": true,
                "type": "builtin",
                "timeout": 300,
                "bundled": true
            },
            {
                "id": "bad",
                "name": "Bad Extension",
                "description": "This one is broken",
                "enabled": false,
                "type": "http",
                "uri": "https://example.com",
                "timeout": 300,
                "bundled": true
            }
        ]"#,
        );
        let result = validate_bundled_extensions(f.path()).unwrap();
        assert_eq!(result.total, 2);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].index, 1);
        assert_eq!(result.errors[0].id, "bad");
    }

    #[test]
    fn test_empty_array_is_valid() {
        let f = write_json("[]");
        let result = validate_bundled_extensions(f.path()).unwrap();
        assert!(result.is_ok());
        assert_eq!(result.total, 0);
    }

    #[test]
    fn test_valid_types_all_deserialize() {
        for type_name in ExtensionConfig::VALID_TYPES {
            let json = serde_json::json!({
                "type": type_name,
                "name": "test",
                "description": "test",
                "cmd": "echo",
                "args": [],
                "uri": "https://example.com",
                "code": "print('hi')",
                "tools": [],
            });
            let result = serde_json::from_value::<ExtensionConfig>(json);
            assert!(
                result.is_ok(),
                "VALID_TYPES contains \"{}\" but it failed to deserialize: {}",
                type_name,
                result.unwrap_err()
            );
        }
    }
}
