//! Image capability - read images from files or capture from screen
//!
//! Provides a unified `read_image` tool that can:
//! - Read image files from the filesystem
//! - Capture specific windows by fuzzy title matching
//! - Capture full screen (display)
//!
//! Images are automatically resized to optimize for LLM consumption:
//! - Max 1568px on long edge (Anthropic constraint)
//! - Max ~1.15 megapixels total
//! - Output as PNG for quality

use base64::{engine::general_purpose::STANDARD, Engine};
use rmcp::model::{CallToolResult, Content, Role};
use schemars::JsonSchema;
use serde::Deserialize;
use std::io::Cursor;
use std::path::Path;
use xcap::{image as img, Monitor, Window};

/// Maximum pixels on the long edge (Anthropic recommendation)
const MAX_LONG_EDGE: u32 = 1568;

/// Maximum total pixels (~1.15 megapixels, Anthropic recommendation)
const MAX_TOTAL_PIXELS: u32 = 1_150_000;

/// Minimum edge size (below this, quality degrades)
const MIN_EDGE: u32 = 200;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReadImageParams {
    /// Image source. Can be:
    /// - A file path (e.g., "/path/to/image.png")
    /// - A window title or substring (e.g., "Firefox", "Terminal")
    /// - Omit or empty for full screen capture
    #[serde(default)]
    pub source: Option<String>,

    /// Display number for full screen capture (default: 0, the main display)
    #[serde(default)]
    pub display: Option<u64>,
}

pub struct ImageTool;

impl ImageTool {
    pub fn new() -> Self {
        Self
    }

    pub fn read_image(&self, params: ReadImageParams) -> CallToolResult {
        let source = params.source.as_deref().unwrap_or("").trim();

        if source.is_empty() {
            // Full screen capture
            self.capture_display(params.display.unwrap_or(0))
        } else if looks_like_path(source) {
            // File path
            self.read_file(source)
        } else {
            // Window title (fuzzy match)
            self.capture_window(source)
        }
    }

    fn read_file(&self, path: &str) -> CallToolResult {
        let path = Path::new(path);

        if !path.exists() {
            return CallToolResult::error(vec![Content::text(format!(
                "File not found: {}",
                path.display()
            ))]);
        }

        let image = match img::open(path) {
            Ok(i) => i,
            Err(e) => {
                return CallToolResult::error(vec![Content::text(format!(
                    "Failed to open image: {}",
                    e
                ))]);
            }
        };

        self.process_and_encode(image, &format!("Read image from {}", path.display()))
    }

    fn capture_window(&self, query: &str) -> CallToolResult {
        let windows = match Window::all() {
            Ok(w) => w,
            Err(e) => {
                return CallToolResult::error(vec![Content::text(format!(
                    "Failed to list windows: {}",
                    e
                ))]);
            }
        };

        // Filter to windows with non-empty titles, excluding common system UI
        // Collect (window, title) pairs where title is successfully retrieved
        let candidates: Vec<_> = windows
            .iter()
            .filter_map(|w| {
                w.title().ok().and_then(|title| {
                    if !title.is_empty() && !is_system_ui(&title) {
                        Some((w, title))
                    } else {
                        None
                    }
                })
            })
            .collect();

        // Fuzzy match: case-insensitive substring
        let query_lower = query.to_lowercase();
        let matches: Vec<_> = candidates
            .iter()
            .filter(|(_, title)| title.to_lowercase().contains(&query_lower))
            .collect();

        match matches.len() {
            0 => {
                // No match - list available windows
                let available: Vec<_> =
                    candidates.iter().map(|(_, title)| title.as_str()).collect();
                let list = if available.is_empty() {
                    "No windows available".to_string()
                } else {
                    available.join("\n  ")
                };
                CallToolResult::error(vec![Content::text(format!(
                    "No window matching '{}'. Available windows:\n  {}",
                    query, list
                ))])
            }
            1 => {
                // Exact match - capture it
                let (window, title) = matches[0];
                match window.capture_image() {
                    Ok(captured) => {
                        let dynamic = img::DynamicImage::ImageRgba8(captured);
                        self.process_and_encode(dynamic, &format!("Captured window '{}'", title))
                    }
                    Err(e) => CallToolResult::error(vec![Content::text(format!(
                        "Failed to capture window '{}': {}",
                        title, e
                    ))]),
                }
            }
            _ => {
                // Multiple matches - ask for clarification
                let titles: Vec<_> = matches.iter().map(|(_, title)| title.as_str()).collect();
                CallToolResult::error(vec![Content::text(format!(
                    "Multiple windows match '{}'. Be more specific:\n  {}",
                    query,
                    titles.join("\n  ")
                ))])
            }
        }
    }

    fn capture_display(&self, display_num: u64) -> CallToolResult {
        let monitors = match Monitor::all() {
            Ok(m) => m,
            Err(e) => {
                return CallToolResult::error(vec![Content::text(format!(
                    "Failed to access monitors: {}",
                    e
                ))]);
            }
        };

        let monitor = match monitors.get(display_num as usize) {
            Some(m) => m,
            None => {
                return CallToolResult::error(vec![Content::text(format!(
                    "Display {} not found. Available: 0-{}",
                    display_num,
                    monitors.len().saturating_sub(1)
                ))]);
            }
        };

        match monitor.capture_image() {
            Ok(captured) => {
                let dynamic = img::DynamicImage::ImageRgba8(captured);
                self.process_and_encode(dynamic, &format!("Captured display {}", display_num))
            }
            Err(e) => CallToolResult::error(vec![Content::text(format!(
                "Failed to capture display {}: {}",
                display_num, e
            ))]),
        }
    }

    fn process_and_encode(&self, image: img::DynamicImage, description: &str) -> CallToolResult {
        let (width, height) = (image.width(), image.height());

        // Calculate resize dimensions
        let (new_width, new_height) = calculate_resize_dimensions(width, height);

        // Resize if needed
        let processed = if new_width != width || new_height != height {
            img::DynamicImage::ImageRgba8(img::imageops::resize(
                &image,
                new_width,
                new_height,
                img::imageops::FilterType::Lanczos3,
            ))
        } else {
            image
        };

        // Encode to PNG
        let mut bytes = Vec::new();
        if let Err(e) = processed.write_to(&mut Cursor::new(&mut bytes), img::ImageFormat::Png) {
            return CallToolResult::error(vec![Content::text(format!(
                "Failed to encode image: {}",
                e
            ))]);
        }

        // Base64 encode
        let base64_data = STANDARD.encode(&bytes);

        // Return with text for assistant and image with priority 0.0
        CallToolResult::success(vec![
            Content::text(description).with_audience(vec![Role::Assistant]),
            Content::image(base64_data, "image/png").with_priority(0.0),
        ])
    }
}

impl Default for ImageTool {
    fn default() -> Self {
        Self::new()
    }
}

/// Calculate target dimensions respecting both constraints:
/// 1. Long edge <= MAX_LONG_EDGE
/// 2. Total pixels <= MAX_TOTAL_PIXELS
fn calculate_resize_dimensions(width: u32, height: u32) -> (u32, u32) {
    // Don't upscale tiny images
    if width < MIN_EDGE && height < MIN_EDGE {
        return (width, height);
    }

    let mut w = width as f64;
    let mut h = height as f64;

    // Constraint 1: Long edge
    let long_edge = w.max(h);
    if long_edge > MAX_LONG_EDGE as f64 {
        let scale = MAX_LONG_EDGE as f64 / long_edge;
        w *= scale;
        h *= scale;
    }

    // Constraint 2: Total pixels
    let total_pixels = w * h;
    if total_pixels > MAX_TOTAL_PIXELS as f64 {
        let scale = (MAX_TOTAL_PIXELS as f64 / total_pixels).sqrt();
        w *= scale;
        h *= scale;
    }

    // Round to nearest pixel, ensure at least 1
    ((w.round() as u32).max(1), (h.round() as u32).max(1))
}

/// Check if a string looks like a file path
fn looks_like_path(s: &str) -> bool {
    s.starts_with('/')
        || s.starts_with("~/")
        || s.starts_with("./")
        || s.starts_with("../")
        || s.contains('/')
        || s.ends_with(".png")
        || s.ends_with(".jpg")
        || s.ends_with(".jpeg")
        || s.ends_with(".gif")
        || s.ends_with(".webp")
        || s.ends_with(".bmp")
}

/// Check if a window title looks like system UI (menubar items, etc.)
fn is_system_ui(title: &str) -> bool {
    // Common macOS system UI patterns
    title == "Menubar"
        || title == "Clock"
        || title == "Battery"
        || title == "WiFi"
        || title == "ScreenMirroring"
        || title.starts_with("Item-")
        || title.contains("StatusBarItem")
        || title.contains("BentoBox")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_resize_no_change_small() {
        // Small image, no resize needed
        assert_eq!(calculate_resize_dimensions(800, 600), (800, 600));
    }

    #[test]
    fn test_calculate_resize_long_edge() {
        // 1920x1080 -> scale to fit 1568 long edge
        let (w, h) = calculate_resize_dimensions(1920, 1080);
        assert!(w <= MAX_LONG_EDGE);
        assert!(h <= MAX_LONG_EDGE);
        // Should preserve aspect ratio (16:9)
        let ratio = w as f64 / h as f64;
        assert!((ratio - 16.0 / 9.0).abs() < 0.01);
    }

    #[test]
    fn test_calculate_resize_megapixel_limit() {
        // Square image that exceeds megapixel limit
        // 1568x1568 = 2.46MP, should be scaled down
        let (w, h) = calculate_resize_dimensions(1568, 1568);
        let pixels = w * h;
        assert!(pixels <= MAX_TOTAL_PIXELS);
    }

    #[test]
    fn test_calculate_resize_retina_display() {
        // Typical MacBook Retina: 3024x1964
        let (w, h) = calculate_resize_dimensions(3024, 1964);
        assert!(w <= MAX_LONG_EDGE);
        assert!(h <= MAX_LONG_EDGE);
        assert!(w * h <= MAX_TOTAL_PIXELS);
    }

    #[test]
    fn test_calculate_resize_tiny_image() {
        // Very small image, don't touch it
        assert_eq!(calculate_resize_dimensions(100, 100), (100, 100));
    }

    #[test]
    fn test_looks_like_path() {
        assert!(looks_like_path("/absolute/path.png"));
        assert!(looks_like_path("~/home/image.jpg"));
        assert!(looks_like_path("./relative/path.png"));
        assert!(looks_like_path("../parent/image.png"));
        assert!(looks_like_path("some/path/image.png"));
        assert!(looks_like_path("screenshot.png"));

        assert!(!looks_like_path("Firefox"));
        assert!(!looks_like_path("Terminal"));
        assert!(!looks_like_path("My Window Title"));
    }

    #[test]
    fn test_is_system_ui() {
        assert!(is_system_ui("Menubar"));
        assert!(is_system_ui("Clock"));
        assert!(is_system_ui("Battery"));
        assert!(is_system_ui("Item-0"));
        assert!(is_system_ui("TupleStatusBarItem"));
        assert!(is_system_ui("BentoBox-0"));

        assert!(!is_system_ui("Firefox"));
        assert!(!is_system_ui("~/Development/goose"));
        assert!(!is_system_ui("Terminal"));
    }
}
