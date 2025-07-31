use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs;
use tokio::io::AsyncWriteExt;

use crate::ai_training::{TrainingDataset, TrainingPair};
use crate::core::types::Diagnostic;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExportFormat {
    JsonLines,   // For streaming ML pipelines
    Parquet,     // For efficient columnar storage
    HuggingFace, // For HF datasets format
    OpenAI,      // For OpenAI fine-tuning format
    Custom,      // For custom formats
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MLTrainingExample {
    pub prompt: String,
    pub completion: String,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIFineTuneFormat {
    pub messages: Vec<OpenAIMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HuggingFaceFormat {
    pub text: String,
    pub label: String,
    pub features: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct ParquetCompatibleRecord<'a> {
    id: &'a str,
    before_code: &'a str,
    after_code: &'a str,
    diagnostics: String,
    fix_description: &'a str,
    confidence: String,
    language: &'a str,
    file_path: &'a str,
    timestamp: i64,
    context: Option<String>,
}

#[derive(Debug, Serialize)]
struct ParquetSchemaInfo {
    format_version: &'static str,
    columns: Vec<ColumnInfo>,
    compression: &'static str,
    row_count: usize,
}

#[derive(Debug, Serialize)]
struct ColumnInfo {
    name: &'static str,
    data_type: &'static str,
    nullable: bool,
}

pub struct TrainingExporter {
    format: ExportFormat,
    include_context: bool,
    include_metadata: bool,
    max_context_tokens: usize,
}

impl TrainingExporter {
    pub fn new(format: ExportFormat) -> Self {
        Self {
            format,
            include_context: true,
            include_metadata: true,
            max_context_tokens: 2000,
        }
    }

    pub fn with_context(mut self, include: bool) -> Self {
        self.include_context = include;
        self
    }

    pub fn with_metadata(mut self, include: bool) -> Self {
        self.include_metadata = include;
        self
    }

    pub fn with_max_tokens(mut self, max_tokens: usize) -> Self {
        self.max_context_tokens = max_tokens;
        self
    }

    pub async fn export_dataset(
        &self,
        dataset: &TrainingDataset,
        output_path: &Path,
    ) -> Result<()> {
        match self.format {
            ExportFormat::JsonLines => self.export_jsonl(dataset, output_path).await,
            ExportFormat::OpenAI => self.export_openai(dataset, output_path).await,
            ExportFormat::HuggingFace => self.export_huggingface(dataset, output_path).await,
            ExportFormat::Parquet => self.export_parquet(dataset, output_path).await,
            ExportFormat::Custom => self.export_custom(dataset, output_path).await,
        }
    }

    async fn export_jsonl(&self, dataset: &TrainingDataset, output_path: &Path) -> Result<()> {
        let mut file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(output_path)
            .await
            .context("Failed to create output file")?;

        for pair in &dataset.pairs {
            let example = self.create_ml_example(pair)?;
            let json = serde_json::to_string(&example)?;
            file.write_all(json.as_bytes()).await?;
            file.write_all(b"\n").await?;
        }

        file.flush().await?;
        Ok(())
    }

    async fn export_openai(&self, dataset: &TrainingDataset, output_path: &Path) -> Result<()> {
        let mut file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(output_path)
            .await
            .context("Failed to create output file")?;

        for pair in &dataset.pairs {
            let format = self.create_openai_format(pair)?;
            let json = serde_json::to_string(&format)?;
            file.write_all(json.as_bytes()).await?;
            file.write_all(b"\n").await?;
        }

        file.flush().await?;
        Ok(())
    }

    async fn export_huggingface(
        &self,
        dataset: &TrainingDataset,
        output_path: &Path,
    ) -> Result<()> {
        let mut file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(output_path)
            .await
            .context("Failed to create output file")?;

        for pair in &dataset.pairs {
            let format = self.create_huggingface_format(pair)?;
            let json = serde_json::to_string(&format)?;
            file.write_all(json.as_bytes()).await?;
            file.write_all(b"\n").await?;
        }

        file.flush().await?;
        Ok(())
    }

    async fn export_parquet(&self, dataset: &TrainingDataset, output_path: &Path) -> Result<()> {
        // Export as a compressed JSON Lines file that can be easily converted to Parquet
        // This provides similar functionality without the arrow dependency issues
        
        let mut file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(output_path)
            .await
            .context("Failed to create output file")?;

        // Write a header comment explaining the format
        let header = "# LSPbridge Training Dataset - Parquet-compatible JSON Lines format\n\
                     # Each line contains a JSON object with the following fields:\n\
                     # - id, before_code, after_code, diagnostics, fix_description, \n\
                     # - confidence, language, file_path, timestamp, context\n\
                     # This can be converted to Parquet using pyarrow or similar tools\n";
        file.write_all(header.as_bytes()).await?;

        // Process data in batches for efficiency
        const BATCH_SIZE: usize = 100;
        let mut buffer = String::with_capacity(1024 * 1024); // 1MB buffer
        
        for (idx, pair) in dataset.pairs.iter().enumerate() {
            // Create a structured record that mimics Parquet columnar format
            let record = ParquetCompatibleRecord {
                id: &pair.id,
                before_code: &pair.before_code,
                after_code: &pair.after_code,
                diagnostics: serde_json::to_string(&pair.diagnostics)?,
                fix_description: &pair.fix_description,
                confidence: format!("{:?}", pair.confidence),
                language: &pair.language,
                file_path: &pair.file_path,
                timestamp: pair.timestamp.timestamp_millis(),
                context: if self.include_context {
                    Some(serde_json::to_string(&pair.context)?)
                } else {
                    None
                },
            };
            
            // Serialize and append to buffer
            let json = serde_json::to_string(&record)?;
            buffer.push_str(&json);
            buffer.push('\n');
            
            // Write buffer when it reaches batch size or at the end
            if (idx + 1) % BATCH_SIZE == 0 || idx == dataset.pairs.len() - 1 {
                file.write_all(buffer.as_bytes()).await?;
                buffer.clear();
            }
        }
        
        file.flush().await?;
        
        // Write metadata file for schema information
        let metadata_path = output_path.with_extension("parquet.schema");
        let schema_info = ParquetSchemaInfo {
            format_version: "1.0",
            columns: vec![
                ColumnInfo { name: "id", data_type: "string", nullable: false },
                ColumnInfo { name: "before_code", data_type: "string", nullable: false },
                ColumnInfo { name: "after_code", data_type: "string", nullable: false },
                ColumnInfo { name: "diagnostics", data_type: "json", nullable: false },
                ColumnInfo { name: "fix_description", data_type: "string", nullable: false },
                ColumnInfo { name: "confidence", data_type: "string", nullable: false },
                ColumnInfo { name: "language", data_type: "string", nullable: false },
                ColumnInfo { name: "file_path", data_type: "string", nullable: false },
                ColumnInfo { name: "timestamp", data_type: "int64", nullable: false },
                ColumnInfo { name: "context", data_type: "json", nullable: true },
            ],
            compression: "none",
            row_count: dataset.pairs.len(),
        };
        
        let schema_json = serde_json::to_string_pretty(&schema_info)?;
        fs::write(&metadata_path, schema_json).await?;
        
        println!("Exported {} records to Parquet-compatible format", dataset.pairs.len());
        println!("Schema information saved to: {}", metadata_path.display());
        
        Ok(())
    }

    async fn export_custom(&self, dataset: &TrainingDataset, output_path: &Path) -> Result<()> {
        // Export as comprehensive JSON with all data
        let json = serde_json::to_string_pretty(&dataset)?;
        fs::write(output_path, json).await?;
        Ok(())
    }

    fn create_ml_example(&self, pair: &TrainingPair) -> Result<MLTrainingExample> {
        let prompt = self.build_prompt(pair);
        let completion = self.build_completion(pair);

        let metadata = if self.include_metadata {
            serde_json::json!({
                "language": pair.language,
                "confidence": pair.confidence.score,
                "diagnostic_count": pair.diagnostics.len(),
                "file_path": pair.file_path,
                "timestamp": pair.timestamp,
            })
        } else {
            serde_json::json!({})
        };

        Ok(MLTrainingExample {
            prompt,
            completion,
            metadata,
        })
    }

    fn create_openai_format(&self, pair: &TrainingPair) -> Result<OpenAIFineTuneFormat> {
        let system_message = OpenAIMessage {
            role: "system".to_string(),
            content: "You are an expert programmer who fixes code errors. Analyze the diagnostics and provide the corrected code.".to_string(),
        };

        let user_message = OpenAIMessage {
            role: "user".to_string(),
            content: self.build_prompt(pair),
        };

        let assistant_message = OpenAIMessage {
            role: "assistant".to_string(),
            content: self.build_completion(pair),
        };

        let mut messages = Vec::with_capacity(3);
        messages.push(system_message);
        messages.push(user_message);
        messages.push(assistant_message);

        Ok(OpenAIFineTuneFormat { messages })
    }

    fn create_huggingface_format(&self, pair: &TrainingPair) -> Result<HuggingFaceFormat> {
        let text = format!(
            "### Code with errors:\n{}\n\n### Diagnostics:\n{}\n\n### Fixed code:\n{}",
            pair.before_code,
            self.format_diagnostics(&pair.diagnostics),
            pair.after_code
        );

        let diagnostic_types: Vec<&str> =
            pair.diagnostics.iter().map(|d| d.source.as_str()).collect();

        let features = serde_json::json!({
            "language": pair.language,
            "confidence": pair.confidence.score,
            "diagnostic_types": diagnostic_types,
        });

        Ok(HuggingFaceFormat {
            text,
            label: pair.fix_description.clone(),
            features,
        })
    }

    fn build_prompt(&self, pair: &TrainingPair) -> String {
        // Estimate capacity: language name + diagnostics + code + context
        let estimated_size = 200
            + pair.diagnostics.len() * 100
            + pair.before_code.len()
            + if self.include_context {
                pair.context.surrounding_code.len() * 50
            } else {
                0
            };

        let mut prompt = String::with_capacity(estimated_size);
        prompt.push_str(&format!(
            "Fix the following {} code that has these diagnostics:\n\n",
            pair.language
        ));

        // Add diagnostics
        prompt.push_str("Diagnostics:\n");
        for diag in &pair.diagnostics {
            prompt.push_str(&format!(
                "- Line {}: {} - {}\n",
                diag.range.start.line, diag.severity, diag.message
            ));
        }

        // Add code
        prompt.push_str("\nCode:\n```");
        prompt.push_str(&pair.language);
        prompt.push('\n');
        prompt.push_str(&pair.before_code);
        prompt.push_str("\n```\n");

        // Add context if enabled
        if self.include_context && !pair.context.surrounding_code.is_empty() {
            prompt.push_str("\nContext:\n");

            // Limit context to max tokens
            let mut context_parts = Vec::with_capacity(5);
            for (file, code) in pair.context.surrounding_code.iter().take(5) {
                context_parts.push(format!("File: {}\n{}", file, code));
            }
            let context_str = context_parts.join("\n---\n");

            if context_str.len() > self.max_context_tokens {
                prompt.push_str(&context_str[..self.max_context_tokens]);
                prompt.push_str("...\n");
            } else {
                prompt.push_str(&context_str);
            }
        }

        prompt
    }

    fn build_completion(&self, pair: &TrainingPair) -> String {
        // Estimate capacity: fix description + code + formatting
        let estimated_size = pair.fix_description.len() + pair.after_code.len() + 50;
        let mut completion = String::with_capacity(estimated_size);

        // Add fix description if available
        if !pair.fix_description.is_empty() {
            completion.push_str(&format!("Fix: {}\n\n", pair.fix_description));
        }

        // Add the fixed code
        completion.push_str("```");
        completion.push_str(&pair.language);
        completion.push('\n');
        completion.push_str(&pair.after_code);
        completion.push_str("\n```");

        completion
    }

    fn format_diagnostics(&self, diagnostics: &[Diagnostic]) -> String {
        let estimated_size = diagnostics.len() * 80; // Average diagnostic line length
        let mut result = String::with_capacity(estimated_size);

        for (i, d) in diagnostics.iter().enumerate() {
            if i > 0 {
                result.push('\n');
            }
            result.push_str(&format!(
                "{}: {} at line {}",
                d.severity, d.message, d.range.start.line
            ));
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::semantic_context::SemanticContext;
    use crate::core::types::{DiagnosticSeverity, Position, Range};

    #[tokio::test]
    async fn test_jsonl_export() {
        let mut dataset = TrainingDataset::new(
            "Test Dataset".to_string(),
            "Test dataset for export".to_string(),
        );

        let diagnostic = Diagnostic::new(
            "test.ts".to_string(),
            Range {
                start: Position {
                    line: 10,
                    character: 5,
                },
                end: Position {
                    line: 10,
                    character: 15,
                },
            },
            DiagnosticSeverity::Error,
            "Type error".to_string(),
            "typescript".to_string(),
        );

        let pair = TrainingPair::new(
            "const x: number = \"string\"".to_string(),
            "const x: string = \"string\"".to_string(),
            vec![diagnostic],
            SemanticContext::default(),
            "typescript".to_string(),
        )
        .with_confidence(0.95);

        dataset.add_pair(pair);

        let exporter = TrainingExporter::new(ExportFormat::JsonLines);
        let temp_file = tempfile::NamedTempFile::new().unwrap();

        exporter
            .export_dataset(&dataset, temp_file.path())
            .await
            .unwrap();

        let content = tokio::fs::read_to_string(temp_file.path()).await.unwrap();
        assert!(!content.is_empty());
        assert!(content.contains("Type error"));
    }
}
