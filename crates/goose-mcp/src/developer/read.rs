use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use base64::{engine::general_purpose::STANDARD, Engine};
use image::{imageops::FilterType, io::Reader as ImageReader, DynamicImage, ImageFormat};
use rmcp::model::{CallToolResult, Content};
use schemars::JsonSchema;
use serde::Deserialize;

const DEFAULT_MAX_LINES: usize = 2000;
const DEFAULT_MAX_BYTES: usize = 50 * 1024;
const MAX_IMAGE_DIMENSION: u32 = 2000;
const BINARY_SAMPLE_SIZE: usize = 8 * 1024;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReadParams {
    pub path: String,
    #[schemars(description = "0-indexed line offset for text files. offset=0 starts at line 1.")]
    #[serde(default)]
    pub offset: Option<usize>,
    #[schemars(description = "Maximum number of lines to return for text files.")]
    #[serde(default)]
    pub limit: Option<usize>,
}

pub struct ReadTool;

impl ReadTool {
    pub fn new() -> Self {
        Self
    }

    pub fn read(&self, params: ReadParams) -> CallToolResult {
        self.read_with_cwd(params, None)
    }

    pub fn read_with_cwd(&self, params: ReadParams, working_dir: Option<&Path>) -> CallToolResult {
        let path = resolve_path(&params.path, working_dir);
        if !path.exists() {
            return CallToolResult::error(vec![Content::text(format!(
                "File not found: {}",
                path.display()
            ))]);
        }

        if !path.is_file() {
            return CallToolResult::error(vec![Content::text(format!(
                "Path is not a file: {}",
                path.display()
            ))]);
        }

        if looks_like_image(&path) {
            return read_image(&path);
        }

        read_text(&path, params.offset, params.limit)
    }
}

impl Default for ReadTool {
    fn default() -> Self {
        Self::new()
    }
}

fn resolve_path(path: &str, working_dir: Option<&Path>) -> PathBuf {
    let path = PathBuf::from(path);
    if path.is_absolute() {
        path
    } else {
        working_dir
            .map(Path::to_path_buf)
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("."))
            .join(path)
    }
}

fn looks_like_image(path: &Path) -> bool {
    ImageReader::open(path)
        .ok()
        .and_then(|reader| reader.with_guessed_format().ok())
        .and_then(|reader| reader.format())
        .is_some()
}

fn read_image(path: &Path) -> CallToolResult {
    let reader = match ImageReader::open(path).and_then(|reader| reader.with_guessed_format()) {
        Ok(reader) => reader,
        Err(error) => {
            return CallToolResult::error(vec![Content::text(format!(
                "Failed to open image: {}",
                error
            ))])
        }
    };

    let image = match reader.decode() {
        Ok(image) => image,
        Err(error) => {
            return CallToolResult::error(vec![Content::text(format!(
                "Failed to decode image: {}",
                error
            ))])
        }
    };

    let original_width = image.width();
    let original_height = image.height();
    let resized = resize_image(image);

    let mut encoded = Vec::new();
    if let Err(error) = resized.write_to(&mut Cursor::new(&mut encoded), ImageFormat::Png) {
        return CallToolResult::error(vec![Content::text(format!(
            "Failed to encode image: {}",
            error
        ))]);
    }

    let resized_note = if resized.width() != original_width || resized.height() != original_height {
        let scale = original_width as f64 / resized.width() as f64;
        format!(
            "\n[Image: original {}x{}, displayed at {}x{}. Multiply coordinates by {:.2} to map to original.]",
            original_width,
            original_height,
            resized.width(),
            resized.height(),
            scale
        )
    } else {
        String::new()
    };

    CallToolResult::success(vec![
        Content::text(format!("Read image file [image/png]{}", resized_note)),
        Content::image(STANDARD.encode(encoded), "image/png").with_priority(0.0),
    ])
}

fn resize_image(image: DynamicImage) -> DynamicImage {
    let width = image.width();
    let height = image.height();
    if width <= MAX_IMAGE_DIMENSION && height <= MAX_IMAGE_DIMENSION {
        return image;
    }

    let ratio =
        (MAX_IMAGE_DIMENSION as f64 / width as f64).min(MAX_IMAGE_DIMENSION as f64 / height as f64);
    let new_width = (width as f64 * ratio).round().max(1.0) as u32;
    let new_height = (height as f64 * ratio).round().max(1.0) as u32;
    image.resize(new_width, new_height, FilterType::Lanczos3)
}

fn read_text(path: &Path, offset: Option<usize>, limit: Option<usize>) -> CallToolResult {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => {
            return CallToolResult::error(vec![Content::text(format!(
                "Failed to read {}: {}",
                path.display(),
                error
            ))])
        }
    };

    if bytes.is_empty() {
        return CallToolResult::success(vec![Content::text("File is empty.".to_string())]);
    }

    if is_probably_binary(&bytes) {
        return CallToolResult::error(vec![Content::text(format!(
            "Refusing to read binary file as text: {}",
            path.display()
        ))]);
    }

    let content = String::from_utf8_lossy(&bytes).into_owned();
    let all_lines: Vec<&str> = content.lines().collect();
    let total_lines = all_lines.len();

    let start_index = offset.unwrap_or(0);
    if start_index >= total_lines {
        return CallToolResult::error(vec![Content::text(format!(
            "Offset {} is beyond end of file ({} lines total). Offsets are 0-indexed.",
            start_index, total_lines
        ))]);
    }

    let selected = if let Some(limit) = limit {
        let end = (start_index + limit).min(total_lines);
        all_lines[start_index..end].join("\n")
    } else {
        all_lines[start_index..].join("\n")
    };

    let truncation = truncate_head(&selected, DEFAULT_MAX_LINES, DEFAULT_MAX_BYTES);

    let mut output = if truncation.content.is_empty() {
        "(no output)".to_string()
    } else {
        truncation.content
    };

    if truncation.first_line_exceeds_limit {
        let first_line_size = all_lines
            .get(start_index)
            .map(|line| line.len())
            .unwrap_or_default();
        let line_number = start_index + 1;
        output = format!(
            "[Line {} is {}, exceeds {} limit. Use shell: sed -n '{}p' {} | head -c {}]",
            line_number,
            format_size(first_line_size),
            format_size(DEFAULT_MAX_BYTES),
            line_number,
            path.display(),
            DEFAULT_MAX_BYTES
        );
    } else if truncation.truncated {
        let end_index = start_index + truncation.output_lines.saturating_sub(1);
        let start_line = start_index + 1;
        let end_line = end_index + 1;
        let next_offset = end_index + 1;
        if truncation.truncated_by == Some(TruncatedBy::Lines) {
            output.push_str(&format!(
                "\n\n[Showing lines {}-{} of {}. Offsets are 0-indexed; use offset={} to continue.]",
                start_line, end_line, total_lines, next_offset
            ));
        } else {
            output.push_str(&format!(
                "\n\n[Showing lines {}-{} of {} ({} limit). Offsets are 0-indexed; use offset={} to continue.]",
                start_line,
                end_line,
                total_lines,
                format_size(DEFAULT_MAX_BYTES),
                next_offset
            ));
        }
    } else if let Some(limit) = limit {
        let end = (start_index + limit).min(total_lines);
        if end < total_lines {
            output.push_str(&format!(
                "\n\n[{} more lines in file. Offsets are 0-indexed; use offset={} to continue.]",
                total_lines - end,
                end
            ));
        }
    }

    CallToolResult::success(vec![Content::text(output)])
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TruncatedBy {
    Lines,
    Bytes,
}

#[derive(Debug)]
struct TruncationResult {
    content: String,
    truncated: bool,
    truncated_by: Option<TruncatedBy>,
    output_lines: usize,
    first_line_exceeds_limit: bool,
}

fn truncate_head(content: &str, max_lines: usize, max_bytes: usize) -> TruncationResult {
    let total_bytes = content.len();
    let lines: Vec<&str> = content.split('\n').collect();
    let total_lines = lines.len();

    if total_lines <= max_lines && total_bytes <= max_bytes {
        return TruncationResult {
            content: content.to_string(),
            truncated: false,
            truncated_by: None,
            output_lines: total_lines,
            first_line_exceeds_limit: false,
        };
    }

    if !lines.is_empty() && lines[0].len() > max_bytes {
        return TruncationResult {
            content: String::new(),
            truncated: true,
            truncated_by: Some(TruncatedBy::Bytes),
            output_lines: 0,
            first_line_exceeds_limit: true,
        };
    }

    let mut output_lines = Vec::new();
    let mut output_bytes = 0usize;
    let mut truncated_by = TruncatedBy::Lines;

    for (index, line) in lines.iter().enumerate().take(max_lines) {
        let line_bytes = line.len() + usize::from(index > 0);
        if output_bytes + line_bytes > max_bytes {
            truncated_by = TruncatedBy::Bytes;
            break;
        }

        output_lines.push(*line);
        output_bytes += line_bytes;
    }

    let content = output_lines.join("\n");
    TruncationResult {
        content,
        truncated: true,
        truncated_by: Some(truncated_by),
        output_lines: output_lines.len(),
        first_line_exceeds_limit: false,
    }
}

fn format_size(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{}B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1}KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1}MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

fn is_probably_binary(bytes: &[u8]) -> bool {
    let sample_len = bytes.len().min(BINARY_SAMPLE_SIZE);
    let sample = &bytes[..sample_len];

    if sample.contains(&0) {
        return true;
    }

    let control_bytes = sample
        .iter()
        .filter(|byte| !matches!(**byte, b'\n' | b'\r' | b'\t' | 0x20..=0x7e | 0x80..=0xff))
        .count();

    control_bytes * 10 > sample_len * 3
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::model::RawContent;

    fn extract_text(result: &CallToolResult) -> Option<&str> {
        result
            .content
            .iter()
            .find_map(|content| match &content.raw {
                RawContent::Text(text) => Some(text.text.as_str()),
                _ => None,
            })
    }

    #[test]
    fn read_text_with_offset_and_limit() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        fs::write(&file, "a\nb\nc\nd\n").unwrap();

        let tool = ReadTool::new();
        let result = tool.read(ReadParams {
            path: file.display().to_string(),
            offset: Some(1),
            limit: Some(2),
        });

        let text = extract_text(&result).unwrap();
        assert!(text.starts_with("b\nc"));
        assert!(text.contains("use offset=3 to continue."));
    }

    #[test]
    fn read_offset_out_of_bounds() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        fs::write(&file, "line\n").unwrap();

        let tool = ReadTool::new();
        let result = tool.read(ReadParams {
            path: file.display().to_string(),
            offset: Some(10),
            limit: None,
        });

        assert_eq!(result.is_error, Some(true));
    }

    #[test]
    fn read_image_returns_image_content() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("pixel.png");
        let image = DynamicImage::new_rgba8(1, 1);
        image.save(&file).unwrap();

        let tool = ReadTool::new();
        let result = tool.read(ReadParams {
            path: file.display().to_string(),
            offset: None,
            limit: None,
        });

        assert_eq!(result.is_error, Some(false));
        assert!(result
            .content
            .iter()
            .any(|item| matches!(item.raw, RawContent::Image(_))));
    }

    #[test]
    fn read_empty_file_reports_empty() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("empty.txt");
        fs::write(&file, "").unwrap();

        let tool = ReadTool::new();
        let result = tool.read(ReadParams {
            path: file.display().to_string(),
            offset: None,
            limit: None,
        });

        assert_eq!(result.is_error, Some(false));
        let text = extract_text(&result).unwrap();
        assert_eq!(text, "File is empty.");
    }

    #[test]
    fn read_binary_file_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("binary.bin");
        fs::write(&file, [0_u8, 159, 146, 150]).unwrap();

        let tool = ReadTool::new();
        let result = tool.read(ReadParams {
            path: file.display().to_string(),
            offset: None,
            limit: None,
        });

        assert_eq!(result.is_error, Some(true));
        let text = extract_text(&result).unwrap();
        assert!(text.contains("binary file"));
    }

    #[test]
    fn read_resolves_relative_paths_from_working_dir() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("nested.txt"), "from cwd").unwrap();

        let tool = ReadTool::new();
        let result = tool.read_with_cwd(
            ReadParams {
                path: "nested.txt".to_string(),
                offset: None,
                limit: None,
            },
            Some(dir.path()),
        );

        assert_eq!(result.is_error, Some(false));
        let text = extract_text(&result).unwrap();
        assert_eq!(text, "from cwd");
    }
}
