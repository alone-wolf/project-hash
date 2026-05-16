use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Clone, Parser)]
#[command(name = "project-hash")]
#[command(about = "Compute a stable hash for a configured unit input set")]
#[command(version)]
pub struct Cli {
    #[arg(short = 'c', long = "config", value_name = "PATH")]
    pub config: Option<PathBuf>,

    #[arg(
        short = 'u',
        long = "unit",
        value_name = "NAME",
        required_unless_present = "init"
    )]
    pub unit: Option<String>,

    #[arg(
        long = "init",
        conflicts_with_all = ["config", "unit", "json", "list_files", "explain"]
    )]
    pub init: bool,

    #[arg(long = "json")]
    pub json: bool,

    #[arg(long = "list-files")]
    pub list_files: bool,

    #[arg(long = "explain")]
    pub explain: bool,
}
