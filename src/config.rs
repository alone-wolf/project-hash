use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ConfigFile {
    pub version: serde_yaml::Value,
    pub units: BTreeMap<String, UnitConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UnitConfig {
    pub root: PathBuf,
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ResolvedUnitConfig {
    pub name: String,
    pub root: PathBuf,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
}

pub const DEFAULT_CONFIG_FILE_NAME: &str = "project-hash.yaml";

const SAMPLE_CONFIG: &str = r#"version: 1
units:
  web-ui:
    root: ./apps/web-ui
    include:
      - "src/**/*"
      - "public/**/*"
      - "package.json"
      - "pnpm-lock.yaml"
      - "tsconfig.json"
      - "vite.config.*"
    exclude:
      - "dist/**/*"
      - "coverage/**/*"
      - "**/*.tmp"

  api-server:
    root: ./services/api-server
    include:
      - "src/**/*.rs"
      - "Cargo.toml"
      - "Cargo.lock"
      - "build.rs"
      - "migrations/**/*"
    exclude:
      - "target/**/*"
      - "**/*.tmp"

  docs:
    root: ./docs
    include:
      - "**/*.md"
      - ".vitepress/**/*"
      - "package.json"
    exclude:
      - ".cache/**/*"
      - "dist/**/*"
"#;

pub fn load_config(path: &Path) -> Result<ConfigFile> {
    if !path.exists() {
        return Err(anyhow!("config file not found: {}", path.display()));
    }

    let contents = fs::read_to_string(path)
        .with_context(|| format!("failed to read config file: {}", path.display()))?;

    let config = serde_yaml::from_str::<ConfigFile>(&contents)
        .with_context(|| format!("failed to parse YAML config: {}", path.display()))?;

    Ok(config)
}

pub fn initialize_sample_config() -> Result<PathBuf> {
    let current_dir = std::env::current_dir().context("failed to determine current directory")?;
    initialize_sample_config_in_dir(&current_dir)
}

pub fn initialize_sample_config_in_dir(directory: &Path) -> Result<PathBuf> {
    let config_path = directory.join(DEFAULT_CONFIG_FILE_NAME);

    if config_path.exists() {
        return Err(anyhow!(
            "config file already exists: {}",
            config_path.display()
        ));
    }

    fs::write(&config_path, SAMPLE_CONFIG).with_context(|| {
        format!(
            "failed to write sample config file: {}",
            config_path.display()
        )
    })?;

    Ok(config_path)
}

pub fn resolve_unit(
    config: &ConfigFile,
    config_path: &Path,
    unit_name: &str,
) -> Result<ResolvedUnitConfig> {
    let unit = config
        .units
        .get(unit_name)
        .ok_or_else(|| anyhow!("unit '{}' not found in config", unit_name))?;

    let config_dir = config_path.parent().unwrap_or_else(|| Path::new("."));
    let root = if unit.root.is_absolute() {
        unit.root.clone()
    } else {
        config_dir.join(&unit.root)
    };

    Ok(ResolvedUnitConfig {
        name: unit_name.to_owned(),
        root,
        include: unit.include.clone(),
        exclude: unit.exclude.clone(),
    })
}
