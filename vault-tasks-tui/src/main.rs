use std::io;

use clap::{CommandFactory, Parser};
use clap_complete::generate;
use cli::Cli;
use color_eyre::Result;
use config::Config;
use tracing::debug;
use vault_tasks_core::{TaskManager, init_logging};

use crate::app::App;

mod action;
mod app;
mod cli;
mod components;
mod config;
mod errors;

mod time_management;
mod tui;
mod widgets;

#[tokio::main]
async fn main() -> Result<()> {
    crate::errors::init()?;
    init_logging()?;

    let args = Cli::parse();

    let config = Config::new(&args)?;
    debug!("Config loaded: {:#?}", config);
    match args.command {
        Some(cli::Commands::GenerateConfig { path }) => Config::generate_config(path),
        Some(cli::Commands::Stdout) => {
            let task_mgr = TaskManager::load_from_config(&config.core)?;
            println!("{}", task_mgr.tasks);
            Ok(())
        }
        Some(cli::Commands::NewTask {
            tasks,
            filename: filename_opt,
        }) => {
            let mut task_mgr = TaskManager::load_from_config(&config.core)?;
            tasks
                .iter()
                .for_each(|task| task_mgr.add_task(task, filename_opt.clone()));
            Ok(())
        }
        Some(cli::Commands::GenerateCompletions { shell }) => {
            generate(shell, &mut Cli::command(), "vault-tasks", &mut io::stdout());
            Ok(())
        }
        Some(cli::Commands::Fix) => {
            TaskManager::load_from_config(&config.core)?;
            Ok(())
        }
        _ => {
            let mut app = App::new(&args)?;
            app.run().await
        }
    }
}
