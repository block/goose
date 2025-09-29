use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Parameters for the analyze tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AnalyzeParams {
    /// Absolute path. Step 1: Directory for overview. Step 2: File for details. Step 3: Directory with focus param for call graphs
    pub path: String,

    /// Symbol name for call graph analysis (Step 3). Requires directory path with broad enough scope to capture all relevant symbol references
    pub focus: Option<String>,

    /// Call graph depth. 0=where defined, 1=direct callers/callees, 2+=transitive chains
    #[serde(default = "default_follow_depth")]
    pub follow_depth: u32,

    /// Directory recursion limit. 0=unlimited (warning: fails on binary files)
    #[serde(default = "default_max_depth")]
    pub max_depth: u32,

    /// Allow large outputs without warning (default: false)
    #[serde(default)]
    pub force: bool,
}

fn default_follow_depth() -> u32 {
    2
}

fn default_max_depth() -> u32 {
    3
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub functions: Vec<FunctionInfo>,
    pub classes: Vec<ClassInfo>,
    pub imports: Vec<String>,
    // Semantic analysis fields
    pub calls: Vec<CallInfo>,
    pub references: Vec<ReferenceInfo>,
    // Structure mode fields (for compact overview)
    pub function_count: usize,
    pub class_count: usize,
    pub line_count: usize,
    pub import_count: usize,
    pub main_line: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionInfo {
    pub name: String,
    pub line: usize,
    pub params: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassInfo {
    pub name: String,
    pub line: usize,
    pub methods: Vec<FunctionInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallInfo {
    pub caller_name: Option<String>, // Function containing this call
    pub callee_name: String,         // Function being called
    pub line: usize,
    pub column: usize,
    pub context: String, // Line of code containing the call
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceInfo {
    pub symbol: String,
    pub ref_type: ReferenceType,
    pub line: usize,
    pub context: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReferenceType {
    Definition,
    Call,
    Import,
    Assignment,
}

// Entry type for directory results - cleaner than overloading AnalysisResult
#[derive(Debug, Clone)]
pub enum EntryType {
    File(AnalysisResult),
    Directory,
    SymlinkDir(PathBuf),
    SymlinkFile(PathBuf),
}

// Type alias for complex query results
pub type ElementQueryResult = (Vec<FunctionInfo>, Vec<ClassInfo>, Vec<String>);

#[derive(Debug, Clone)]
pub struct CallChain {
    pub path: Vec<(PathBuf, usize, String, String)>, // (file, line, from, to)
}

// Data structure to pass to format_focused_output_with_chains
pub struct FocusedAnalysisData<'a> {
    pub focus_symbol: &'a str,
    pub follow_depth: u32,
    pub files_analyzed: &'a [PathBuf],
    pub definitions: &'a [(PathBuf, usize)],
    pub incoming_chains: &'a [CallChain],
    pub outgoing_chains: &'a [CallChain],
}

/// Analysis modes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnalysisMode {
    Structure, // Directory overview
    Semantic,  // File details
    Focused,   // Symbol tracking
}

impl AnalysisMode {
    pub fn as_str(&self) -> &str {
        match self {
            AnalysisMode::Structure => "structure",
            AnalysisMode::Semantic => "semantic",
            AnalysisMode::Focused => "focused",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "structure" => AnalysisMode::Structure,
            "semantic" => AnalysisMode::Semantic,
            "focused" => AnalysisMode::Focused,
            _ => AnalysisMode::Structure,
        }
    }
}

impl AnalysisResult {
    /// Create an empty analysis result with only line count
    pub fn empty(line_count: usize) -> Self {
        Self {
            functions: vec![],
            classes: vec![],
            imports: vec![],
            calls: vec![],
            references: vec![],
            function_count: 0,
            class_count: 0,
            line_count,
            import_count: 0,
            main_line: None,
        }
    }
}
