use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::config::{get_config_dir, get_data_dir};

#[derive(Parser, Debug)]
#[command(author, version = version(), about)]
pub struct Cli {
    /// Vault to open (can be a single file or a directory)
    pub vault_path: Option<PathBuf>,

    /// Tick rate, i.e. number of ticks per second
    #[arg(short, long, value_name = "FLOAT", default_value_t = 4.0)]
    pub tick_rate: f64,

    /// Frame rate, i.e. number of frames per second
    #[arg(short, long, value_name = "FLOAT", default_value_t = 60.0)]
    pub frame_rate: f64,

    /// Use a custom config file
    #[arg(short, long, value_name = "PATH")]
    pub config_path: Option<PathBuf>,

    /// Optional subcommand to run
    #[command(subcommand)]
    pub command: Option<Commands>,
}
#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// Open explorer view
    #[command(alias = "test")]
    Explorer,
    /// Open filter view
    Filter,
    /// Generates a new configuration file from the default one
    GenerateConfig { path: Option<PathBuf> },
}

const VERSION_MESSAGE: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    "-",
    env!("VERGEN_GIT_DESCRIBE"),
    " (",
    env!("VERGEN_BUILD_DATE"),
    ")"
);

pub fn version() -> String {
    let author = clap::crate_authors!();

    // let current_exe_path = PathBuf::from(clap::crate_name!()).display().to_string();
    let config_dir_path = get_config_dir().display().to_string();
    let data_dir_path = get_data_dir().display().to_string();

    format!(
        "\
{VERSION_MESSAGE}

Authors: {author}

Config directory: {config_dir_path}
Data directory: {data_dir_path}"
    )
}
