mod cli;
mod config;
mod errors;

use clap::Parser;
use cli::Cli;
use color_eyre::Result;
use config::Config;
use vault_tasks_core::init_logging;

fn main() -> Result<()> {
    crate::errors::init()?;
    init_logging()?;

    let cli = Cli::parse();
    println!("{cli:#?}");
    let config = Config::new(&args)?;
    println!("{:#?}", config);
    Ok(())
}
