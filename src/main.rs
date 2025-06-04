use core::TaskManager;

use clap::Parser;
use cli::Cli;
use color_eyre::{owo_colors::OwoColorize, Result};
use config::Config;
use tracing::debug;

use crate::app::App;

mod action;
mod app;
mod cli;
mod components;
mod config;
mod errors;
mod logging;

mod core;
mod time_management;
mod tui;
mod widgets;

#[tokio::main]
async fn main() -> Result<()> {
    crate::errors::init()?;
    crate::logging::init()?;

    let args = Cli::parse();

    let config = Config::new(&args)?;
    match args.command {
        Some(cli::Commands::GenerateConfig { path }) => Config::generate_config(path),
        Some(cli::Commands::Stdout) => {
            let task_mgr = TaskManager::load_from_config(&config.tasks_config)?;
            println!("{}", task_mgr.tasks);
            Ok(())
        }
        Some(cli::Commands::NewTask {
            description: task,
            path: path_opt,
        }) => {
            let mut task_mgr = TaskManager::load_from_config(&config.tasks_config)?;
            let mut path = path_opt
                .unwrap_or(config.tasks_config.tasks_drop_file)
                .split('/')
                .map(str::to_owned)
                .collect::<Vec<String>>();

            debug!("Adding new task: {} to path: {:?}", task, path);
            task_mgr.add_task(&task, &path)
        }
        _ => {
            let mut app = App::new(&args)?;
            app.run().await
        }
    }
}
