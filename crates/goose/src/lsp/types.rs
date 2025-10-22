use lsp_types::{Diagnostic, Position, Range};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspPosition {
    pub line: u32,
    pub character: u32,
}

impl From<Position> for LspPosition {
    fn from(pos: Position) -> Self {
        Self {
            line: pos.line,
            character: pos.character,
        }
    }
}

impl From<LspPosition> for Position {
    fn from(pos: LspPosition) -> Self {
        Position::new(pos.line, pos.character)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspRange {
    pub start: LspPosition,
    pub end: LspPosition,
}

impl From<Range> for LspRange {
    fn from(range: Range) -> Self {
        Self {
            start: range.start.into(),
            end: range.end.into(),
        }
    }
}

impl From<LspRange> for Range {
    fn from(range: LspRange) -> Self {
        Range::new(range.start.into(), range.end.into())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspDiagnostic {
    pub range: LspRange,
    pub severity: Option<u32>,
    pub code: Option<String>,
    pub source: Option<String>,
    pub message: String,
}

impl From<Diagnostic> for LspDiagnostic {
    fn from(diag: Diagnostic) -> Self {
        Self {
            range: diag.range.into(),
            severity: diag.severity.map(|s| match s {
                lsp_types::DiagnosticSeverity::ERROR => 1,
                lsp_types::DiagnosticSeverity::WARNING => 2,
                lsp_types::DiagnosticSeverity::INFORMATION => 3,
                lsp_types::DiagnosticSeverity::HINT => 4,
                _ => 0,
            }),
            code: diag.code.map(|c| match c {
                lsp_types::NumberOrString::Number(n) => n.to_string(),
                lsp_types::NumberOrString::String(s) => s,
            }),
            source: diag.source,
            message: diag.message,
        }
    }
}
