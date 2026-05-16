use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use globset::{Glob, GlobMatcher};
use walkdir::WalkDir;

use crate::config::ResolvedUnitConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitFile {
    pub absolute_path: PathBuf,
    pub relative_path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileScanStatus {
    Included,
    Excluded,
    Unmatched,
}

impl FileScanStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Included => "included",
            Self::Excluded => "excluded",
            Self::Unmatched => "unmatched",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileScanEntry {
    pub absolute_path: PathBuf,
    pub relative_path: String,
    pub status: FileScanStatus,
    pub include_matches: Vec<String>,
    pub exclude_matches: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanExplanation {
    pub root: PathBuf,
    pub include_patterns: Vec<String>,
    pub exclude_patterns: Vec<String>,
    pub entries: Vec<FileScanEntry>,
}

impl ScanExplanation {
    pub fn scanned_file_count(&self) -> usize {
        self.entries.len()
    }

    pub fn included_count(&self) -> usize {
        self.count_by_status(FileScanStatus::Included)
    }

    pub fn excluded_count(&self) -> usize {
        self.count_by_status(FileScanStatus::Excluded)
    }

    pub fn unmatched_count(&self) -> usize {
        self.count_by_status(FileScanStatus::Unmatched)
    }

    pub fn included_files(&self) -> Vec<UnitFile> {
        self.entries
            .iter()
            .filter(|entry| entry.status == FileScanStatus::Included)
            .map(|entry| UnitFile {
                absolute_path: entry.absolute_path.clone(),
                relative_path: entry.relative_path.clone(),
            })
            .collect()
    }

    fn count_by_status(&self, status: FileScanStatus) -> usize {
        self.entries
            .iter()
            .filter(|entry| entry.status == status)
            .count()
    }
}

pub fn scan_unit_files(unit: &ResolvedUnitConfig) -> Result<Vec<UnitFile>> {
    Ok(explain_unit_files(unit)?.included_files())
}

pub fn explain_unit_files(unit: &ResolvedUnitConfig) -> Result<ScanExplanation> {
    if !unit.root.exists() {
        return Err(anyhow!("unit root does not exist: {}", unit.root.display()));
    }

    if !unit.root.is_dir() {
        return Err(anyhow!(
            "unit root is not a directory: {}",
            unit.root.display()
        ));
    }

    let include_matchers = build_matchers(&unit.include, "include")?;
    let exclude_matchers = build_matchers(&unit.exclude, "exclude")?;
    let mut entries = Vec::new();

    for entry in WalkDir::new(&unit.root).follow_links(false) {
        let entry =
            entry.with_context(|| format!("failed to walk unit root: {}", unit.root.display()))?;

        if !entry.file_type().is_file() {
            continue;
        }

        let relative_path = entry
            .path()
            .strip_prefix(&unit.root)
            .map(normalize_relative_path)
            .with_context(|| {
                format!(
                    "failed to resolve relative path for file: {}",
                    entry.path().display()
                )
            })?;

        let include_matches = matched_patterns(&include_matchers, &relative_path);
        let exclude_matches = if include_matches.is_empty() {
            Vec::new()
        } else {
            matched_patterns(&exclude_matchers, &relative_path)
        };
        let status = if include_matches.is_empty() {
            FileScanStatus::Unmatched
        } else if exclude_matches.is_empty() {
            FileScanStatus::Included
        } else {
            FileScanStatus::Excluded
        };

        entries.push(FileScanEntry {
            absolute_path: entry.path().to_path_buf(),
            relative_path,
            status,
            include_matches,
            exclude_matches,
        });
    }

    entries.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));

    Ok(ScanExplanation {
        root: unit.root.clone(),
        include_patterns: unit.include.clone(),
        exclude_patterns: unit.exclude.clone(),
        entries,
    })
}

struct CompiledPattern {
    pattern: String,
    matcher: GlobMatcher,
}

fn build_matchers(patterns: &[String], group_name: &str) -> Result<Vec<CompiledPattern>> {
    patterns
        .iter()
        .map(|pattern| {
            let glob = Glob::new(pattern)
                .with_context(|| format!("invalid {} glob pattern '{}'", group_name, pattern))?;

            Ok(CompiledPattern {
                pattern: pattern.clone(),
                matcher: glob.compile_matcher(),
            })
        })
        .collect()
}

fn matched_patterns(matchers: &[CompiledPattern], path: &str) -> Vec<String> {
    matchers
        .iter()
        .filter(|matcher| matcher.matcher.is_match(path))
        .map(|matcher| matcher.pattern.clone())
        .collect()
}

fn normalize_relative_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
