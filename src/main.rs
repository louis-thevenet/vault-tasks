use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use config::Config;
use scanner::Scanner;

mod config;
mod scanner;

#[derive(Debug, clap::Parser)]
struct Args {
    #[arg(short, long)]
    config_path: Option<PathBuf>,
}

fn main() -> Result<()> {
    env_logger::init();
    let config = Config::load_config(&Args::parse())?;
    let scanner = Scanner::new(config);
    scanner.scan_vault()?;
    Ok(())
}
