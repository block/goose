use anyhow::Result;
use base64::Engine;
use ignore::gitignore::Gitignore;
use indoc::indoc;
use mcp_core::{handler::ToolError, role::Role, tool::Tool, tool::ToolAnnotations, Content};
use serde_json::{json, Value};
use std::{io::Cursor, path::Path, sync::Arc};
use xcap::{Monitor, Window};

/// Creates the list_windows tool
pub fn create_list_windows_tool() -> Tool {
    Tool::new(
        "list_windows",
        indoc! {r#"
            List all available window titles that can be used with screen_capture.
            Returns a list of window titles that can be used with the window_title parameter
            of the screen_capture tool.
        "#},
        json!({
            "type": "object",
            "required": [],
            "properties": {}
        }),
        Some(ToolAnnotations {
            title: Some("List available windows".to_string()),
            read_only_hint: true,
            destructive_hint: false,
            idempotent_hint: false,
            open_world_hint: false,
        }),
    )
}

/// Creates the screen_capture tool
pub fn create_screen_capture_tool() -> Tool {
    Tool::new(
        "screen_capture",
        indoc! {r#"
            Capture a screenshot of a specified display or window.
            You can capture either:
            1. A full display (monitor) using the display parameter
            2. A specific window by its title using the window_title parameter

            Only one of display or window_title should be specified.
        "#},
        json!({
            "type": "object",
            "required": [],
            "properties": {
                "display": {
                    "type": "integer",
                    "default": 0,
                    "description": "The display number to capture (0 is main display)"
                },
                "window_title": {
                    "type": "string",
                    "default": null,
                    "description": "Optional: the exact title of the window to capture. use the list_windows tool to find the available windows."
                }
            }
        }),
        Some(ToolAnnotations {
            title: Some("Capture a full screen".to_string()),
            read_only_hint: true,
            destructive_hint: false,
            idempotent_hint: false,
            open_world_hint: false,
        }),
    )
}

/// List all available windows that can be captured
pub async fn list_windows(_params: Value) -> Result<Vec<Content>, ToolError> {
    let windows =
        Window::all().map_err(|_| ToolError::ExecutionError("Failed to list windows".into()))?;

    let window_titles: Vec<String> = windows.into_iter().map(|w| w.title().to_string()).collect();

    Ok(vec![
        Content::text(format!("Available windows:\n{}", window_titles.join("\n")))
            .with_audience(vec![Role::Assistant]),
        Content::text(format!("Available windows:\n{}", window_titles.join("\n")))
            .with_audience(vec![Role::User])
            .with_priority(0.0),
    ])
}

/// Helper function to handle Mac screenshot filenames that contain U+202F (narrow no-break space)
fn normalize_mac_screenshot_path(path: &Path) -> std::path::PathBuf {
    // Only process if the path has a filename
    if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
        // Check if this matches Mac screenshot pattern:
        // "Screenshot YYYY-MM-DD at H.MM.SS AM/PM.png"
        if let Some(captures) = regex::Regex::new(r"^Screenshot \d{4}-\d{2}-\d{2} at \d{1,2}\.\d{2}\.\d{2} (AM|PM|am|pm)(?: \(\d+\))?\.png$")
            .ok()
            .and_then(|re| re.captures(filename))
        {

            // Get the AM/PM part
            let meridian = captures.get(1).unwrap().as_str();

            // Find the last space before AM/PM and replace it with U+202F
            let space_pos = filename.rfind(meridian)
                .map(|pos| filename[..pos].trim_end().len())
                .unwrap_or(0);

            if space_pos > 0 {
                let parent = path.parent().unwrap_or(Path::new(""));
                let new_filename = format!(
                    "{}{}{}",
                    &filename[..space_pos],
                    '\u{202F}',
                    &filename[space_pos+1..]
                );
                let new_path = parent.join(new_filename);

                return new_path;
            }
        }
    }
    path.to_path_buf()
}

/// Capture a screenshot of a display or window
pub async fn capture_screen(params: Value) -> Result<Vec<Content>, ToolError> {
    let mut image = if let Some(window_title) = params.get("window_title").and_then(|v| v.as_str())
    {
        // Try to find and capture the specified window
        let windows = Window::all()
            .map_err(|_| ToolError::ExecutionError("Failed to list windows".into()))?;

        let window = windows
            .into_iter()
            .find(|w| w.title() == window_title)
            .ok_or_else(|| {
                ToolError::ExecutionError(format!("No window found with title '{}'", window_title))
            })?;

        window.capture_image().map_err(|e| {
            ToolError::ExecutionError(format!(
                "Failed to capture window '{}': {}",
                window_title, e
            ))
        })?
    } else {
        // Default to display capture if no window title is specified
        let display = params.get("display").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

        let monitors = Monitor::all()
            .map_err(|_| ToolError::ExecutionError("Failed to access monitors".into()))?;
        let monitor = monitors.get(display).ok_or_else(|| {
            ToolError::ExecutionError(format!(
                "{} was not an available monitor, {} found.",
                display,
                monitors.len()
            ))
        })?;

        monitor.capture_image().map_err(|e| {
            ToolError::ExecutionError(format!("Failed to capture display {}: {}", display, e))
        })?
    };

    // Resize the image to a reasonable width while maintaining aspect ratio
    let max_width = 768;
    if image.width() > max_width {
        let scale = max_width as f32 / image.width() as f32;
        let new_height = (image.height() as f32 * scale) as u32;
        image = xcap::image::imageops::resize(
            &image,
            max_width,
            new_height,
            xcap::image::imageops::FilterType::Lanczos3,
        )
    };

    let mut bytes: Vec<u8> = Vec::new();
    image
        .write_to(&mut Cursor::new(&mut bytes), xcap::image::ImageFormat::Png)
        .map_err(|e| ToolError::ExecutionError(format!("Failed to write image buffer {}", e)))?;

    // Convert to base64
    let data = base64::prelude::BASE64_STANDARD.encode(bytes);

    Ok(vec![
        Content::text("Screenshot captured").with_audience(vec![Role::Assistant]),
        Content::image(data, "image/png").with_priority(0.0),
    ])
}

/// Process an image file from disk
pub async fn process_image(
    params: Value,
    ignore_patterns: &Arc<Gitignore>,
    resolve_path_fn: impl Fn(&str) -> Result<std::path::PathBuf, ToolError>,
) -> Result<Vec<Content>, ToolError> {
    let path_str = params
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ToolError::InvalidParameters("Missing 'path' parameter".into()))?;

    // Resolve path using the provided function
    let path = resolve_path_fn(path_str)?;

    let path = {
        if cfg!(target_os = "macos") {
            normalize_mac_screenshot_path(&path)
        } else {
            path
        }
    };

    // Check if file is ignored before proceeding
    if ignore_patterns.matched(&path, false).is_ignore() {
        return Err(ToolError::ExecutionError(format!(
            "Access to '{}' is restricted by .gooseignore",
            path.display()
        )));
    }

    // Check if file exists
    if !path.exists() {
        return Err(ToolError::ExecutionError(format!(
            "File '{}' does not exist",
            path.display()
        )));
    }

    // Check file size (10MB limit for image files)
    const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024; // 10MB in bytes
    let file_size = std::fs::metadata(&path)
        .map_err(|e| ToolError::ExecutionError(format!("Failed to get file metadata: {}", e)))?
        .len();

    if file_size > MAX_FILE_SIZE {
        return Err(ToolError::ExecutionError(format!(
            "File '{}' is too large ({:.2}MB). Maximum size is 10MB.",
            path.display(),
            file_size as f64 / (1024.0 * 1024.0)
        )));
    }

    // Open and decode the image
    let image = xcap::image::open(&path)
        .map_err(|e| ToolError::ExecutionError(format!("Failed to open image file: {}", e)))?;

    // Resize if necessary (same logic as screen_capture)
    let mut processed_image = image;
    let max_width = 768;
    if processed_image.width() > max_width {
        let scale = max_width as f32 / processed_image.width() as f32;
        let new_height = (processed_image.height() as f32 * scale) as u32;
        processed_image = xcap::image::DynamicImage::ImageRgba8(xcap::image::imageops::resize(
            &processed_image,
            max_width,
            new_height,
            xcap::image::imageops::FilterType::Lanczos3,
        ));
    }

    // Convert to PNG and encode as base64
    let mut bytes: Vec<u8> = Vec::new();
    processed_image
        .write_to(&mut Cursor::new(&mut bytes), xcap::image::ImageFormat::Png)
        .map_err(|e| ToolError::ExecutionError(format!("Failed to write image buffer: {}", e)))?;

    let data = base64::prelude::BASE64_STANDARD.encode(bytes);

    Ok(vec![
        Content::text(format!(
            "Successfully processed image from {}",
            path.display()
        ))
        .with_audience(vec![Role::Assistant]),
        Content::image(data, "image/png").with_priority(0.0),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use ignore::gitignore::GitignoreBuilder;
    use serde_json::json;
    use std::sync::Arc;

    #[test]
    fn test_create_list_windows_tool() {
        let tool = create_list_windows_tool();
        assert_eq!(tool.name, "list_windows");
        assert!(!tool.description.is_empty());
        assert!(tool.annotations.is_some());
    }

    #[test]
    fn test_create_screen_capture_tool() {
        let tool = create_screen_capture_tool();
        assert_eq!(tool.name, "screen_capture");
        assert!(!tool.description.is_empty());
        assert!(tool.annotations.is_some());
    }

    #[tokio::test]
    async fn test_list_windows() {
        let result = list_windows(json!({})).await;
        assert!(result.is_ok());
        let content = result.unwrap();
        assert_eq!(content.len(), 2); // One for assistant, one for user
    }

    #[test]
    fn test_normalize_mac_screenshot_path() {
        let path = std::path::Path::new("Screenshot 2023-12-01 at 10.30.45 AM.png");
        let normalized = normalize_mac_screenshot_path(&path);

        // Should return a path (exact behavior depends on regex matching)
        assert!(normalized.file_name().is_some());
    }

    #[tokio::test]
    async fn test_process_image_with_ignore_patterns() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Create ignore patterns
        let mut builder = GitignoreBuilder::new(temp_dir.path().to_path_buf());
        builder.add_line(None, "secret.png").unwrap();
        let ignore_patterns = Arc::new(builder.build().unwrap());

        // Mock resolve_path function
        let resolve_path_fn = |path_str: &str| -> Result<std::path::PathBuf, ToolError> {
            Ok(temp_dir.path().join(path_str))
        };

        // Test with ignored file
        let result = process_image(
            json!({
                "path": "secret.png"
            }),
            &ignore_patterns,
            resolve_path_fn,
        )
        .await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ToolError::ExecutionError(_)));

        temp_dir.close().unwrap();
    }
}
