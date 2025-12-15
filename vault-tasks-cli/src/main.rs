mod cli;
mod config;
mod errors;

use std::path::{Path, PathBuf};

use clap::Parser;
use cli::Cli;
use color_eyre::{Result, eyre::bail};
use config::Config;
use vault_tasks_core::{TaskManager, init_logging, parser};

fn main() -> Result<()> {
    crate::errors::init()?;
    init_logging()?;

    let args = Cli::parse();
    let config = Config::new(&args)?;
    let mut task_manager = TaskManager::load_from_config(&config.core)?;
    match args.command {
        cli::Commands::List {
            file_selector_args,
            filter_args,
        } => {
            println!(
                "Listing tasks with file selector args: {file_selector_args:?} and filter args: {filter_args:?}"
            );

            println!("{}", task_manager.tasks_refactored);
        }
        cli::Commands::Add { task, args } => {
            if args.path.len() > 1 {
                bail!(
                    "Can't decide where to add new task when multiple paths are provided: {:?}",
                    args.path
                );
            }

            let mut task_input = task.as_str();
            let Some(path) = config.cli.drop_file_path else {
                bail!("No drop file configured, can't add task");
            };
            match parser::task::parse_task(&mut task_input, &path, &config.core) {
                Ok(task) => task_manager.add_task(&task)?,

                Err(e) => bail!("Failed to parse task input: {e}"),
            }
        }

        cli::Commands::Mark {
            new_state,
            file_selector_args,
            filter_args,
        } => println!(
            "Marking tasks with new state: {new_state:?}, file selector args: {file_selector_args:?}, and filter args: {filter_args:?}"
        ),
    }
    Ok(())
}
