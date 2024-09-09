use core::TaskManager;
use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use config::Config;

mod config;
mod core;
pub mod file_entry;
mod parser;
mod task;
mod vault_parser;

#[derive(Debug, clap::Parser)]
pub struct Args {
    #[arg(short, long)]
    config_path: Option<PathBuf>,
}

fn main() -> Result<()> {
    env_logger::init();
    let config = Config::load_config(&Args::parse())?;
    let task_mgr = TaskManager::load_from_config(&config)?;
    println!("{task_mgr}");
    Ok(())
}
