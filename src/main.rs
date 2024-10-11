use clap::Parser;
use cli::Cli;
use color_eyre::Result;
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
mod task_core;
mod tui;
mod widgets;

#[tokio::main]
async fn main() -> Result<()> {
    crate::errors::init()?;
    crate::logging::init()?;

    let args = Cli::parse();

    if let Some(cli::Commands::GenerateConfig { path }) = args.command {
        return Config::generate_config(path);
    }

    debug!("{args:#?}");

    let mut app = App::new(&args)?;
    app.run().await
}
