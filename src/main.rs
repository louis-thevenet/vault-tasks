use core::TaskManager;

use clap::Parser;
use cli::Cli;
use color_eyre::Result;
use config::Config;

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
            tasks,
            filename: filename_opt,
        }) => {
            let mut task_mgr = TaskManager::load_from_config(&config.tasks_config)?;
            tasks
                .iter()
                .for_each(|task| task_mgr.add_task(task, filename_opt.clone()));
            Ok(())
        }
        _ => {
            let mut app = App::new(&args)?;
            app.run().await
        }
    }
}
