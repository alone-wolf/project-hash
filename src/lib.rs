pub mod cli;
pub mod config;
pub mod hasher;
pub mod output;
pub mod scanner;

use anyhow::Result;
use std::path::PathBuf;

use crate::cli::Cli;
use crate::config::{
    DEFAULT_CONFIG_FILE_NAME, initialize_sample_config, load_config, resolve_unit,
};
use crate::hasher::calculate_unit_hash;
use crate::output::render_output;
use crate::scanner::{explain_unit_files, scan_unit_files};

pub fn run(cli: Cli) -> Result<String> {
    if cli.init {
        let config_path = initialize_sample_config()?;
        return Ok(format!("created {}\n", config_path.display()));
    }

    let config_path_buf = cli
        .config
        .clone()
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CONFIG_FILE_NAME));
    let config_path = config_path_buf.as_path();
    let unit_name = cli
        .unit
        .as_deref()
        .expect("clap should require --unit unless --init is used");

    let config = load_config(config_path)?;
    let unit = resolve_unit(&config, config_path, unit_name)?;
    let explanation = cli.explain.then(|| explain_unit_files(&unit)).transpose()?;
    let files = match explanation.as_ref() {
        Some(explanation) => explanation.included_files(),
        None => scan_unit_files(&unit)?,
    };
    let result = calculate_unit_hash(&unit.name, &files)?;

    render_output(
        &result,
        explanation.as_ref(),
        cli.json,
        cli.list_files,
        cli.explain,
    )
}
