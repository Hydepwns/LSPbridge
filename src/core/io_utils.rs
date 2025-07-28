use anyhow::{Context, Result};
use serde::{de::DeserializeOwned, Serialize};
/// Common I/O utilities to reduce duplication across the codebase
///
/// This module provides reusable utilities for common I/O patterns,
/// reducing code duplication and providing consistent error handling.
use std::path::Path;
use tokio::fs;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

/// Export any serializable items to a JSONL (JSON Lines) file
///
/// This replaces the duplicated export patterns in ai_training/export.rs
pub async fn export_jsonl<T, I>(items: I, output_path: &Path) -> Result<()>
where
    T: Serialize,
    I: IntoIterator<Item = T>,
{
    let mut file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(output_path)
        .await
        .with_context(|| format!("Failed to create output file: {}", output_path.display()))?;

    for item in items {
        let json = serde_json::to_string(&item).context("Failed to serialize item to JSON")?;
        file.write_all(json.as_bytes())
            .await
            .context("Failed to write JSON to file")?;
        file.write_all(b"\n")
            .await
            .context("Failed to write newline")?;
    }

    file.flush().await.context("Failed to flush file")?;

    Ok(())
}

/// Read items from a JSONL file
pub async fn read_jsonl<T>(input_path: &Path) -> Result<Vec<T>>
where
    T: DeserializeOwned,
{
    let file = fs::File::open(input_path)
        .await
        .with_context(|| format!("Failed to open file: {}", input_path.display()))?;

    let reader = BufReader::new(file);
    let mut lines = reader.lines();
    let mut items = Vec::new();

    while let Some(line) = lines.next_line().await? {
        if !line.trim().is_empty() {
            let item: T = serde_json::from_str(&line)
                .with_context(|| format!("Failed to parse JSON line: {}", line))?;
            items.push(item);
        }
    }

    Ok(items)
}

/// Deserialize JSON string with fallback to default value
///
/// This replaces the repeated pattern in multi_repo/registry.rs
pub fn deserialize_or_default<T>(json_str: &str) -> T
where
    T: DeserializeOwned + Default,
{
    serde_json::from_str(json_str).unwrap_or_default()
}

/// Read and parse a JSON file
pub async fn read_json_file<T>(path: &Path) -> Result<T>
where
    T: DeserializeOwned,
{
    let content = fs::read_to_string(path)
        .await
        .with_context(|| format!("Failed to read file: {}", path.display()))?;

    serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse JSON from: {}", path.display()))
}

/// Write a serializable value to a JSON file
pub async fn write_json_file<T>(value: &T, path: &Path) -> Result<()>
where
    T: Serialize,
{
    let json = serde_json::to_string_pretty(value).context("Failed to serialize to JSON")?;

    fs::write(path, json)
        .await
        .with_context(|| format!("Failed to write file: {}", path.display()))?;

    Ok(())
}

/// Common package.json parsing with proper error handling
#[derive(Debug, serde::Deserialize)]
pub struct PackageJson {
    pub name: Option<String>,
    pub version: Option<String>,
    pub workspaces: Option<serde_json::Value>,
    pub dependencies: Option<serde_json::Value>,
    pub dev_dependencies: Option<serde_json::Value>,
    #[serde(flatten)]
    pub other: serde_json::Map<String, serde_json::Value>,
}

impl PackageJson {
    pub async fn from_path(path: &Path) -> Result<Self> {
        read_json_file(path).await
    }

    pub fn is_workspace_root(&self) -> bool {
        self.workspaces.is_some()
    }
}

/// Common Cargo.toml parsing
#[derive(Debug, serde::Deserialize)]
pub struct CargoToml {
    pub package: Option<CargoPackage>,
    pub workspace: Option<CargoWorkspace>,
    pub dependencies: Option<toml::Value>,
}

#[derive(Debug, serde::Deserialize)]
pub struct CargoPackage {
    pub name: String,
    pub version: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct CargoWorkspace {
    pub members: Option<Vec<String>>,
}

impl CargoToml {
    pub async fn from_path(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .await
            .with_context(|| format!("Failed to read Cargo.toml: {}", path.display()))?;

        toml::from_str(&content)
            .with_context(|| format!("Failed to parse Cargo.toml: {}", path.display()))
    }

    pub fn is_workspace_root(&self) -> bool {
        self.workspace.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_jsonl_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.jsonl");

        #[derive(Debug, PartialEq, Serialize, serde::Deserialize)]
        struct TestItem {
            id: u32,
            name: String,
        }

        let items = vec![
            TestItem {
                id: 1,
                name: "First".to_string(),
            },
            TestItem {
                id: 2,
                name: "Second".to_string(),
            },
        ];

        export_jsonl(&items, &file_path).await.unwrap();
        let loaded: Vec<TestItem> = read_jsonl(&file_path).await.unwrap();

        assert_eq!(items, loaded);
    }

    #[test]
    fn test_deserialize_or_default() {
        #[derive(Debug, Default, PartialEq, serde::Deserialize)]
        struct TestStruct {
            value: Option<String>,
        }

        let valid = r#"{"value": "test"}"#;
        let invalid = "not json";

        let result: TestStruct = deserialize_or_default(valid);
        assert_eq!(result.value, Some("test".to_string()));

        let fallback: TestStruct = deserialize_or_default(invalid);
        assert_eq!(fallback, TestStruct::default());
    }
}
