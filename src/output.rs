use std::fmt::Write;

use anyhow::Result;
use serde::Serialize;

use crate::hasher::UnitHash;
use crate::scanner::{FileScanEntry, FileScanStatus, ScanExplanation};

#[derive(Debug, Serialize)]
struct JsonOutput<'a> {
    unit: &'a str,
    hash: &'a str,
    file_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    files: Option<&'a [String]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    explain: Option<JsonExplainOutput<'a>>,
}

#[derive(Debug, Serialize)]
struct JsonExplainOutput<'a> {
    root: String,
    scanned_file_count: usize,
    included_count: usize,
    excluded_count: usize,
    unmatched_count: usize,
    entries: Vec<JsonExplainEntry<'a>>,
}

#[derive(Debug, Serialize)]
struct JsonExplainEntry<'a> {
    path: &'a str,
    status: &'static str,
    include_matches: &'a [String],
    exclude_matches: &'a [String],
}

pub fn render_output(
    result: &UnitHash,
    explanation: Option<&ScanExplanation>,
    json: bool,
    list_files: bool,
    explain: bool,
) -> Result<String> {
    if json {
        let payload = JsonOutput {
            unit: &result.unit,
            hash: &result.hash,
            file_count: result.file_count,
            files: list_files.then_some(result.files.as_slice()),
            explain: explain
                .then(|| json_explain_output(explanation.expect("missing explanation"))),
        };

        let rendered = serde_json::to_string(&payload)?;
        return Ok(format!("{rendered}\n"));
    }

    if explain {
        return Ok(render_explain_output(
            result,
            explanation.expect("missing explanation"),
        ));
    }

    if list_files {
        if result.files.is_empty() {
            return Ok(String::new());
        }

        return Ok(format!("{}\n", result.files.join("\n")));
    }

    Ok(format!("{}\n", result.hash))
}

fn json_explain_output(explanation: &ScanExplanation) -> JsonExplainOutput<'_> {
    JsonExplainOutput {
        root: explanation.root.to_string_lossy().into_owned(),
        scanned_file_count: explanation.scanned_file_count(),
        included_count: explanation.included_count(),
        excluded_count: explanation.excluded_count(),
        unmatched_count: explanation.unmatched_count(),
        entries: explanation
            .entries
            .iter()
            .map(|entry| JsonExplainEntry {
                path: &entry.relative_path,
                status: entry.status.as_str(),
                include_matches: &entry.include_matches,
                exclude_matches: &entry.exclude_matches,
            })
            .collect(),
    }
}

fn render_explain_output(result: &UnitHash, explanation: &ScanExplanation) -> String {
    let mut output = String::new();
    let included_entries: Vec<&FileScanEntry> = explanation
        .entries
        .iter()
        .filter(|entry| entry.status == FileScanStatus::Included)
        .collect();
    let excluded_entries: Vec<&FileScanEntry> = explanation
        .entries
        .iter()
        .filter(|entry| entry.status == FileScanStatus::Excluded)
        .collect();
    let unmatched_entries: Vec<&FileScanEntry> = explanation
        .entries
        .iter()
        .filter(|entry| entry.status == FileScanStatus::Unmatched)
        .collect();

    writeln!(&mut output, "unit: {}", result.unit).expect("write to string");
    writeln!(&mut output, "root: {}", explanation.root.display()).expect("write to string");
    writeln!(&mut output, "hash: {}", result.hash).expect("write to string");
    writeln!(&mut output, "file_count: {}", result.file_count).expect("write to string");
    writeln!(
        &mut output,
        "scanned_file_count: {}",
        explanation.scanned_file_count()
    )
    .expect("write to string");
    writeln!(
        &mut output,
        "included_count: {}",
        explanation.included_count()
    )
    .expect("write to string");
    writeln!(
        &mut output,
        "excluded_count: {}",
        explanation.excluded_count()
    )
    .expect("write to string");
    writeln!(
        &mut output,
        "unmatched_count: {}",
        explanation.unmatched_count()
    )
    .expect("write to string");

    write_pattern_section(
        &mut output,
        "include_patterns",
        &explanation.include_patterns,
    );
    write_pattern_section(
        &mut output,
        "exclude_patterns",
        &explanation.exclude_patterns,
    );
    render_file_section(&mut output, "included_files", &included_entries, None);
    render_file_section(&mut output, "excluded_files", &excluded_entries, None);
    render_file_section(
        &mut output,
        "unmatched_files",
        &unmatched_entries,
        Some(100),
    );

    output
}

fn write_pattern_section(output: &mut String, label: &str, patterns: &[String]) {
    writeln!(output, "{label}:").expect("write to string");
    if patterns.is_empty() {
        writeln!(output, "  (none)").expect("write to string");
        return;
    }

    for pattern in patterns {
        writeln!(output, "  - {pattern}").expect("write to string");
    }
}

fn format_matches(matches: &[String]) -> String {
    if matches.is_empty() {
        "none".to_owned()
    } else {
        matches.join(", ")
    }
}

fn render_file_section(
    output: &mut String,
    label: &str,
    entries: &[&FileScanEntry],
    max_entries: Option<usize>,
) {
    writeln!(output, "{label}:").expect("write to string");
    if entries.is_empty() {
        writeln!(output, "  (none)").expect("write to string");
        return;
    }

    let mut entries = entries.to_vec();
    entries.sort_by(|left, right| compare_for_display(&left.relative_path, &right.relative_path));

    let shown = max_entries
        .map(|limit| limit.min(entries.len()))
        .unwrap_or(entries.len());

    for entry in entries.iter().take(shown) {
        writeln!(output, "  - {}", entry.relative_path).expect("write to string");
        writeln!(output, "    status: {}", entry.status.as_str()).expect("write to string");
        writeln!(
            output,
            "    include_matches: {}",
            format_matches(&entry.include_matches)
        )
        .expect("write to string");
        writeln!(
            output,
            "    exclude_matches: {}",
            format_matches(&entry.exclude_matches)
        )
        .expect("write to string");
    }

    if shown < entries.len() {
        writeln!(output, "  ... {} more omitted", entries.len() - shown).expect("write to string");
    }
}

fn compare_for_display(left: &str, right: &str) -> std::cmp::Ordering {
    display_sort_key(left).cmp(&display_sort_key(right))
}

fn display_sort_key(path: &str) -> (bool, usize, &str) {
    (path.starts_with('.'), path.matches('/').count(), path)
}
