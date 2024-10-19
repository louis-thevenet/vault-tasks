use clap::Parser;
use cli::Cli;
use color_eyre::Result;
use config::Config;
use task_core::TaskManager;

use crate::app::App;

mod action;
mod app;
mod cli;
mod components;
mod config;
mod errors;
mod logging;
mod task_core;
mod tui;
mod widgets;

#[tokio::main]
async fn main() -> Result<()> {
    crate::errors::init()?;
    crate::logging::init()?;

    let args = Cli::parse();

    match args.command {
        Some(cli::Commands::GenerateConfig { path }) => Config::generate_config(path),
        Some(cli::Commands::Stdout) => {
            let config = Config::new(&args)?;
            let task_mgr = TaskManager::load_from_config(&config)?;
            println!("{}", task_mgr.tasks);
            Ok(())
        }
        _ => {
            let mut app = App::new(&args)?;
            app.run().await
        }
    }
}
