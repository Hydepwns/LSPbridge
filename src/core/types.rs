use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Position {
    pub line: u32,      // 0-based
    pub character: u32, // 0-based
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Location {
    pub uri: String,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub id: String,
    pub file: String,
    pub range: Range,
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub code: Option<String>,
    pub source: String,
    pub related_information: Option<Vec<RelatedInformation>>,
    pub tags: Option<Vec<DiagnosticTag>>,
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    pub name: String,
    pub root_path: String,
    pub language: Option<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticSnapshot {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub workspace: WorkspaceInfo,
    pub diagnostics: Vec<Diagnostic>,
    pub metadata: SnapshotMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticSummary {
    pub total_diagnostics: usize,
    pub error_count: usize,
    pub warning_count: usize,
    pub info_count: usize,
    pub hint_count: usize,
    pub file_count: usize,
    pub source_breakdown: HashMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawDiagnostics {
    pub source: String,
    pub data: serde_json::Value,
    pub timestamp: DateTime<Utc>,
    pub workspace: Option<WorkspaceInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiagnosticFilter {
    pub severities: Option<Vec<DiagnosticSeverity>>,
    pub sources: Option<Vec<String>>,
    pub file_patterns: Option<Vec<String>>,
    pub exclude_patterns: Option<Vec<String>>,
    pub max_results: Option<usize>,
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
