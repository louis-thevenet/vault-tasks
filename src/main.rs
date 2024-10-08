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
mod task_core;
mod tui;
mod widgets;

#[tokio::main]
async fn main() -> Result<()> {
    crate::errors::init()?;
    crate::logging::init()?;

    let args = Cli::parse();

    if matches!(args.command, Some(cli::Commands::GenerateConfig)) {
        return Config::generate_config();
    }

    let mut app = App::new(args.tick_rate, args.frame_rate)?;
    app.run().await
}
