use std::time::Duration;

use clap::Parser;
use cli::Cli;
use color_eyre::Result;
use config::{Config, MethodSettingsEntry, MethodSettingsValue, MethodsAvailable};
use vault_tasks_core::TaskManager;

use crate::app::App;

mod action;
mod app;
mod cli;
mod components;
mod config;
mod errors;
mod logging;

mod tui;
mod widgets;

#[tokio::main]
async fn main() -> Result<()> {
    crate::errors::init()?;
    crate::logging::init()?;
    let mut config = Config::default();

    config.time_management_methods_settings.insert(
        MethodsAvailable::FlowTime,
        vec![MethodSettingsEntry {
            name: String::from("Break Factor"),
            value: MethodSettingsValue::Int(5),
            hint: String::from("Break time is (focus time) / (break factor)"),
        }],
    );
    config.time_management_methods_settings.insert(
        MethodsAvailable::Pomodoro,
        vec![
            MethodSettingsEntry {
                name: String::from("Focus Time"),
                value: MethodSettingsValue::Duration(Duration::from_secs(60 * 25)),
                hint: String::new(),
            },
            MethodSettingsEntry {
                name: String::from("Short Break Time"),
                value: MethodSettingsValue::Duration(Duration::from_secs(60 * 5)),
                hint: String::new(),
            },
            MethodSettingsEntry {
                name: String::from("Long Break Time"),
                value: MethodSettingsValue::Duration(Duration::from_secs(60 * 15)),
                hint: String::new(),
            },
            MethodSettingsEntry {
                name: String::from("Long Break Interval"),
                value: MethodSettingsValue::Int(4),
                hint: String::from("Short breaks before a long break"),
            },
        ],
    );
    // let stra = config.
    // println!("{stra}");
    Ok(())

    // let args = Cli::parse();

    // match args.command {
    //     Some(cli::Commands::GenerateConfig { path }) => Config::generate_config(path),
    //     Some(cli::Commands::Stdout) => {
    //         let config = Config::new(&args)?;
    //         let task_mgr = TaskManager::load_from_config(&config.tasks_config)?;
    //         println!("{}", task_mgr.tasks);
    //         Ok(())
    //     }
    //     _ => {
    //         let mut app = App::new(&args)?;
    //         app.run().await
    //     }
    // }
}
