use core::TaskManager;
use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use config::Config;

mod config;
mod core;
mod parser;
mod scanner;
mod task;

#[derive(Debug, clap::Parser)]
struct Args {
    #[arg(short, long)]
    config_path: Option<PathBuf>,
}

fn main() -> Result<()> {
    env_logger::init();
    let config = Config::load_config(&Args::parse())?;
    let task_mgr = TaskManager::load_from_config(config)?;
    Ok(())
}
