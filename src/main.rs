use core::TaskManager;
use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use config::Config;
use log::debug;

mod config;
mod core;
mod scanner;

#[derive(Debug, clap::Parser)]
struct Args {
    #[arg(short, long)]
    config_path: Option<PathBuf>,
}

fn main() -> Result<()> {
    env_logger::init();
    let config = Config::load_config(&Args::parse())?;
    let task_mgr = TaskManager::new(config)?;
    Ok(())
}
