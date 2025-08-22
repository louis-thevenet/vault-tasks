mod cli;
mod config;
mod errors;

use clap::Parser;
use cli::Cli;
use color_eyre::{Result, eyre::bail};
use config::Config;
use vault_tasks_core::{TaskManager, init_logging};

fn main() -> Result<()> {
    crate::errors::init()?;
    init_logging()?;

    let args = Cli::parse();
    let config = Config::new(&args)?;
    println!("{config:#?}");
    match args.command {
        cli::Commands::List {
            file_selector_args,
            filter_args,
        } => println!(
            "Listing tasks with file selector args: {file_selector_args:?} and filter args: {filter_args:?}"
        ),
        cli::Commands::Add { task, args } => {
            if args.path.len() > 1 {
                bail!(
                    "Can't decide where to add new task when multiple paths are provided: {:?}",
                    args.path
                );
            }

            
            
            let mut task_manager = TaskManager::load_from_config(&config.core)?;
            task_manager.add_task(
                &task,
                args.path.first().and_then(|path| {
                    // TODO: refactor add_task to take paths
                    path.file_name()
                        .map(|filename| filename.to_string_lossy().into_owned())
                }),
            );
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
