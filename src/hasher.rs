use std::fs::File;
use std::io::Read;
use std::path::Path;

use anyhow::{Context, Result};

use crate::scanner::UnitFile;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitHash {
    pub unit: String,
    pub hash: String,
    pub file_count: usize,
    pub files: Vec<String>,
}

pub fn calculate_unit_hash(unit_name: &str, files: &[UnitFile]) -> Result<UnitHash> {
    let mut ordered_files = files.to_vec();
    ordered_files.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));

    let mut manifest_hasher = blake3::Hasher::new();
    let mut relative_paths = Vec::with_capacity(ordered_files.len());

    for file in ordered_files {
        let content_hash = hash_file_contents(&file.absolute_path)?;
        let relative_path_bytes = file.relative_path.as_bytes();

        manifest_hasher.update(&(relative_path_bytes.len() as u64).to_le_bytes());
        manifest_hasher.update(relative_path_bytes);
        manifest_hasher.update(content_hash.as_bytes());

        relative_paths.push(file.relative_path);
    }

    Ok(UnitHash {
        unit: unit_name.to_owned(),
        hash: manifest_hasher.finalize().to_hex().to_string(),
        file_count: relative_paths.len(),
        files: relative_paths,
    })
}

fn hash_file_contents(path: &Path) -> Result<blake3::Hash> {
    let mut file = File::open(path)
        .with_context(|| format!("failed to open file for hashing: {}", path.display()))?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0_u8; 8 * 1024];

    loop {
        let read = file
            .read(&mut buffer)
            .with_context(|| format!("failed to read file for hashing: {}", path.display()))?;

        if read == 0 {
            break;
        }

        hasher.update(&buffer[..read]);
    }

    Ok(hasher.finalize())
}
