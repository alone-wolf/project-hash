use clap::Parser;
use std::process::ExitCode;

use project_hash::{cli::Cli, run};

fn main() -> ExitCode {
    let cli = Cli::parse();

    match run(cli) {
        Ok(output) => {
            print!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error:#}");
            ExitCode::from(1)
        }
    }
}
