//! JSONL dataset validation for MLX / agent-brain finetune pipelines.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetValidationReport {
    pub valid: bool,
    pub path: String,
    pub files_scanned: u64,
    pub entries: u64,
    pub invalid_lines: u64,
    pub empty_instruction: u64,
    pub empty_response: u64,
    pub min_entries_required: u64,
    pub errors: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct JsonlEntry {
    instruction: Option<String>,
    response: Option<String>,
    text: Option<String>,
    messages: Option<Vec<Message>>,
}

#[derive(Debug, Deserialize)]
struct Message {
    role: Option<String>,
    content: Option<String>,
}

pub fn validate_dataset(path: &Path, min_entries: u64) -> Result<DatasetValidationReport> {
    let mut report = DatasetValidationReport {
        valid: false,
        path: path.display().to_string(),
        files_scanned: 0,
        entries: 0,
        invalid_lines: 0,
        empty_instruction: 0,
        empty_response: 0,
        min_entries_required: min_entries,
        errors: Vec::new(),
    };

    let files = collect_jsonl_files(path)?;
    if files.is_empty() {
        report
            .errors
            .push(format!("no JSONL files found under {}", path.display()));
        return Ok(report);
    }

    for file in files {
        report.files_scanned += 1;
        validate_file(&file, &mut report)?;
    }

    if report.entries < min_entries {
        report.errors.push(format!(
            "dataset has {} entries (minimum {min_entries})",
            report.entries
        ));
    }

    if report.invalid_lines > 0 {
        report.errors.push(format!(
            "{} malformed JSONL line(s) — fix before training",
            report.invalid_lines
        ));
    }

    report.valid = report.errors.is_empty();
    Ok(report)
}

fn collect_jsonl_files(path: &Path) -> Result<Vec<PathBuf>> {
    if path.is_file() {
        return Ok(vec![path.to_path_buf()]);
    }

    let mut files = Vec::new();
    if path.is_dir() {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let p = entry.path();
            if p.is_file() && p.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                files.push(p);
            }
        }
        files.sort();
    }
    Ok(files)
}

fn validate_file(path: &Path, report: &mut DatasetValidationReport) -> Result<()> {
    let file = std::fs::File::open(path)
        .with_context(|| format!("open dataset file {}", path.display()))?;
    let reader = BufReader::new(file);

    for (idx, line) in reader.lines().enumerate() {
        let line = line.with_context(|| format!("read line {} in {}", idx + 1, path.display()))?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let parsed: JsonlEntry = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                report.invalid_lines += 1;
                if report.errors.len() < 5 {
                    report
                        .errors
                        .push(format!("{}:{} invalid JSON: {e}", path.display(), idx + 1));
                }
                continue;
            }
        };

        let (instruction, response) = normalize_entry(parsed);
        if instruction.trim().is_empty() {
            report.empty_instruction += 1;
            continue;
        }
        if response.trim().is_empty() {
            report.empty_response += 1;
            continue;
        }

        report.entries += 1;
    }

    Ok(())
}

fn normalize_entry(entry: JsonlEntry) -> (String, String) {
    if let (Some(i), Some(r)) = (entry.instruction, entry.response) {
        return (i, r);
    }
    if let Some(text) = entry.text {
        return (text.clone(), text);
    }
    if let Some(messages) = entry.messages {
        let mut user = String::new();
        let mut assistant = String::new();
        for msg in messages {
            let role = msg.role.unwrap_or_default().to_lowercase();
            let content = msg.content.unwrap_or_default();
            if role == "user" && user.is_empty() {
                user = content;
            } else if role == "assistant" {
                assistant = content;
            }
        }
        return (user, assistant);
    }
    (String::new(), String::new())
}

pub fn default_merged_dataset() -> PathBuf {
    agent_body_core::memory_dir()
        .join("datasets")
        .join("spine.merged.jsonl")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn accepts_instruction_response_entries() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("train.jsonl");
        std::fs::write(
            &file,
            r#"{"instruction":"fix bug","response":"done","workflow_name":"dev","node_kind":"agent","model":"","outcome":"success","timestamp":"2026-06-20T00:00:00Z"}
"#,
        )
        .unwrap();
        let report = validate_dataset(dir.path(), 1).unwrap();
        assert!(report.valid);
        assert_eq!(report.entries, 1);
    }

    #[test]
    fn rejects_empty_dataset() {
        let dir = TempDir::new().unwrap();
        let report = validate_dataset(dir.path(), 1).unwrap();
        assert!(!report.valid);
    }
}
