use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

/// A position in a text document expressed as zero-based line and character offset.
/// 
/// This follows the LSP specification for position representation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Position {
    /// Zero-based line number
    pub line: u32,
    /// Zero-based character offset within the line
    pub character: u32,
}

/// A range in a text document expressed as start and end positions.
/// 
/// The end position is exclusive, meaning it represents the position immediately after the last character.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Range {
    /// The range's start position
    pub start: Position,
    /// The range's end position (exclusive)
    pub end: Position,
}

/// A location in a document represented by a URI and a range.
/// 
/// Follows the LSP specification for representing a location within a document.
/// The URI can be a file path or any other resource identifier.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Location {
    /// Document URI or file path
    pub uri: String,
    /// Position range within the document
    pub range: Range,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum DiagnosticSeverity {
    Error = 1,
    Warning = 2,
    Information = 3,
    Hint = 4,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DiagnosticTag {
    Unnecessary,
    Deprecated,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RelatedInformation {
    pub location: Location,
    pub message: String,
}

/// A diagnostic message from a language server or analyzer.
/// 
/// Represents an issue found in source code such as errors, warnings, or hints.
/// Contains all information needed to locate and understand the diagnostic.
/// 
/// # Examples
/// 
/// ```rust
/// use lspbridge::core::{Diagnostic, DiagnosticSeverity, Range, Position};
/// 
/// let diagnostic = Diagnostic {
///     id: "rust-001".to_string(),
///     file: "src/main.rs".to_string(),
///     range: Range {
///         start: Position { line: 10, character: 5 },
///         end: Position { line: 10, character: 15 },
///     },
///     severity: DiagnosticSeverity::Error,
///     message: "undefined variable `foo`".to_string(),
///     code: Some("E0425".to_string()),
///     source: "rustc".to_string(),
///     related_information: None,
///     tags: None,
///     data: None,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    /// Unique identifier for this diagnostic
    pub id: String,
    /// File path where the diagnostic was found
    pub file: String,
    /// Location range within the file
    pub range: Range,
    /// Severity level (error, warning, info, hint)
    pub severity: DiagnosticSeverity,
    /// Human-readable diagnostic message
    pub message: String,
    /// Optional error code from the language server
    pub code: Option<String>,
    /// Source of the diagnostic (e.g., "rustc", "typescript", "eslint")
    pub source: String,
    /// Additional related locations and information
    pub related_information: Option<Vec<RelatedInformation>>,
    /// Diagnostic tags (e.g., unnecessary, deprecated)
    pub tags: Option<Vec<DiagnosticTag>>,
    /// Language server specific additional data
    pub data: Option<serde_json::Value>,
}

/// Information about a workspace or project being analyzed.
/// 
/// Contains metadata about the project including its name, location,
/// and primary programming language. Used to provide context in exports.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    /// Human-readable project or workspace name
    pub name: String,
    /// Absolute path to the workspace root directory
    pub root_path: String,
    /// Primary programming language of the project
    pub language: Option<String>,
    /// Project version (from package.json, Cargo.toml, etc.)
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMetadata {
    pub capture_method: CaptureMethod,
    pub editor_info: EditorInfo,
    pub language_servers: Vec<String>,
    pub total_files: usize,
    pub filtered_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CaptureMethod {
    Manual,
    Automatic,
    Scheduled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorInfo {
    pub name: String,
    pub version: String,
}

/// A snapshot of diagnostic data captured at a specific point in time.
/// 
/// Represents a complete capture of all diagnostics from one or more language
/// servers for a workspace. This is the primary data structure used throughout
/// LSPbridge for storing and processing diagnostic information.
/// 
/// # Examples
/// 
/// ```rust
/// use lspbridge::core::{DiagnosticSnapshot, WorkspaceInfo, SnapshotMetadata};
/// use chrono::Utc;
/// use uuid::Uuid;
/// 
/// let snapshot = DiagnosticSnapshot {
///     id: Uuid::new_v4(),
///     timestamp: Utc::now(),
///     workspace: WorkspaceInfo {
///         name: "my-project".to_string(),
///         root_path: "/path/to/project".to_string(),
///         language: Some("rust".to_string()),
///         version: Some("0.1.0".to_string()),
///     },
///     diagnostics: vec![],
///     metadata: SnapshotMetadata { /* ... */ },
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticSnapshot {
    /// Unique identifier for this snapshot
    pub id: Uuid,
    /// When this snapshot was captured
    pub timestamp: DateTime<Utc>,
    /// Information about the workspace/project
    pub workspace: WorkspaceInfo,
    /// All diagnostics captured in this snapshot
    pub diagnostics: Vec<Diagnostic>,
    /// Metadata about how this snapshot was captured
    pub metadata: SnapshotMetadata,
}

/// Statistical summary of diagnostic data.
/// 
/// Provides aggregated counts and breakdowns of diagnostics by severity,
/// source, and file. Useful for generating reports and dashboard views.
/// 
/// # Examples
/// 
/// ```rust
/// use lspbridge::core::DiagnosticSummary;
/// use std::collections::HashMap;
/// 
/// let summary = DiagnosticSummary {
///     total_diagnostics: 42,
///     error_count: 5,
///     warning_count: 15,
///     info_count: 12,
///     hint_count: 10,
///     file_count: 8,
///     source_breakdown: {
///         let mut map = HashMap::new();
///         map.insert("rustc".to_string(), 25);
///         map.insert("clippy".to_string(), 17);
///         map
///     },
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticSummary {
    /// Total number of diagnostics across all severities
    pub total_diagnostics: usize,
    /// Number of error-level diagnostics
    pub error_count: usize,
    /// Number of warning-level diagnostics
    pub warning_count: usize,
    /// Number of information-level diagnostics
    pub info_count: usize,
    /// Number of hint-level diagnostics
    pub hint_count: usize,
    /// Number of unique files with diagnostics
    pub file_count: usize,
    /// Count of diagnostics by source (e.g., "rustc": 25, "clippy": 17)
    pub source_breakdown: HashMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawDiagnostics {
    pub source: String,
    pub data: serde_json::Value,
    pub timestamp: DateTime<Utc>,
    pub workspace: Option<WorkspaceInfo>,
}

/// Filter criteria for querying diagnostic data.
/// 
/// Allows filtering diagnostics by various criteria including severity,
/// source analyzer, file patterns, and time ranges. All filter criteria
/// are optional and will be combined with AND logic when specified.
/// 
/// # Examples
/// 
/// ```rust
/// use lspbridge::core::{DiagnosticFilter, DiagnosticSeverity};
/// use chrono::{Utc, Duration};
/// 
/// // Filter for errors and warnings from rustc in Rust files
/// let filter = DiagnosticFilter {
///     severities: Some(vec![DiagnosticSeverity::Error, DiagnosticSeverity::Warning]),
///     sources: Some(vec!["rustc".to_string()]),
///     file_patterns: Some(vec!["*.rs".to_string()]),
///     exclude_patterns: Some(vec!["target/**".to_string()]),
///     max_results: Some(100),
///     since: Some(Utc::now() - Duration::hours(24)),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiagnosticFilter {
    /// Filter by diagnostic severity levels
    pub severities: Option<Vec<DiagnosticSeverity>>,
    /// Filter by analyzer/language server sources
    pub sources: Option<Vec<String>>,
    /// Include only files matching these glob patterns
    pub file_patterns: Option<Vec<String>>,
    /// Exclude files matching these glob patterns
    pub exclude_patterns: Option<Vec<String>>,
    /// Maximum number of results to return
    pub max_results: Option<usize>,
    /// Only include diagnostics captured after this time
    pub since: Option<DateTime<Utc>>,
}

impl Default for DiagnosticFilter {
    fn default() -> Self {
        Self {
            severities: None,
            sources: None,
            file_patterns: None,
            exclude_patterns: None,
            max_results: None,
            since: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticResult {
    pub diagnostics: HashMap<PathBuf, Vec<Diagnostic>>,
    pub summary: DiagnosticSummary,
    pub timestamp: DateTime<Utc>,
}

impl DiagnosticResult {
    pub fn new() -> Self {
        Self {
            diagnostics: HashMap::new(),
            summary: DiagnosticSummary {
                total_diagnostics: 0,
                error_count: 0,
                warning_count: 0,
                info_count: 0,
                hint_count: 0,
                file_count: 0,
                source_breakdown: HashMap::new(),
            },
            timestamp: Utc::now(),
        }
    }
}

impl Diagnostic {
    pub fn new(
        file: String,
        range: Range,
        severity: DiagnosticSeverity,
        message: String,
        source: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            file,
            range,
            severity,
            message,
            code: None,
            source,
            related_information: None,
            tags: None,
            data: None,
        }
    }
}

impl DiagnosticSnapshot {
    pub fn new(workspace: WorkspaceInfo, diagnostics: Vec<Diagnostic>) -> Self {
        let metadata = SnapshotMetadata {
            capture_method: CaptureMethod::Automatic,
            editor_info: EditorInfo {
                name: "unknown".to_string(),
                version: "unknown".to_string(),
            },
            language_servers: diagnostics
                .iter()
                .map(|d| d.source.clone())
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect(),
            total_files: diagnostics
                .iter()
                .map(|d| &d.file)
                .collect::<std::collections::HashSet<_>>()
                .len(),
            filtered_count: diagnostics.len(),
        };

        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            workspace,
            diagnostics,
            metadata,
        }
    }

    pub fn generate_summary(&self) -> DiagnosticSummary {
        let mut summary = DiagnosticSummary {
            total_diagnostics: self.diagnostics.len(),
            error_count: 0,
            warning_count: 0,
            info_count: 0,
            hint_count: 0,
            file_count: 0,
            source_breakdown: HashMap::new(),
        };

        let mut files = std::collections::HashSet::new();

        for diagnostic in &self.diagnostics {
            files.insert(&diagnostic.file);

            match diagnostic.severity {
                DiagnosticSeverity::Error => summary.error_count += 1,
                DiagnosticSeverity::Warning => summary.warning_count += 1,
                DiagnosticSeverity::Information => summary.info_count += 1,
                DiagnosticSeverity::Hint => summary.hint_count += 1,
            }

            *summary
                .source_breakdown
                .entry(diagnostic.source.clone())
                .or_insert(0) += 1;
        }

        summary.file_count = files.len();
        summary
    }
}

impl std::fmt::Display for DiagnosticSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiagnosticSeverity::Error => write!(f, "Error"),
            DiagnosticSeverity::Warning => write!(f, "Warning"),
            DiagnosticSeverity::Information => write!(f, "Info"),
            DiagnosticSeverity::Hint => write!(f, "Hint"),
        }
    }
}

impl std::str::FromStr for DiagnosticSeverity {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "error" => Ok(DiagnosticSeverity::Error),
            "warning" => Ok(DiagnosticSeverity::Warning),
            "info" | "information" => Ok(DiagnosticSeverity::Information),
            "hint" => Ok(DiagnosticSeverity::Hint),
            _ => Err(format!("Unknown severity: {}", s)),
        }
    }
}
