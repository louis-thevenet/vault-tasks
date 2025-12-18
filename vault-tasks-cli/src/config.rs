#![allow(dead_code)]
use std::{
    env,
    fs::{File, create_dir_all},
    io::Write,
    path::PathBuf,
};

use serde::Deserialize;
use vault_tasks_core::config::{ProtoConfig, TasksConfig};

use color_eyre::{Result, eyre::bail};
use directories::ProjectDirs;
use lazy_static::lazy_static;
use tracing::{debug, info};

use crate::cli::Cli;
const CLI_CONFIG: &str = include_str!("../../.config/cli.toml");
const CONFIG_FILE_NAME: &str = "cli";

lazy_static! {
    pub static ref PROJECT_NAME: String = env!("CARGO_CRATE_NAME").to_uppercase();
    pub static ref DATA_FOLDER: Option<PathBuf> =
        env::var(format!("{}_DATA", PROJECT_NAME.clone()))
            .ok()
            .map(PathBuf::from);
    pub static ref CONFIG_FOLDER: Option<PathBuf> =
        env::var(format!("{}_CONFIG", PROJECT_NAME.clone()))
            .ok()
            .map(PathBuf::from);
}

#[derive(Clone, Debug, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub drop_file_path: Option<PathBuf>,
}
impl Default for AppConfig {
    fn default() -> Self {
        toml::from_str(CLI_CONFIG).unwrap()
    }
}
#[derive(Clone, Debug, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub cli: AppConfig,
    #[serde(default)]
    pub core: TasksConfig,
}
impl Config {
    pub fn new(args: &Cli) -> Result<Self, config::ConfigError> {
        let data_dir = get_data_dir();
        let config_path = args.config_path.clone().unwrap_or_else(get_config_dir);
        debug!(
            "Using data directory at {} and config directory at {}",
            data_dir.display(),
            config_path.display()
        );
        // A config file was provided
        let builder = if config_path.is_file() {
            config::Config::builder()
                .set_default("data_dir", data_dir.to_str().unwrap())?
                .add_source(config::File::from(config_path.clone()))
        } else {
            let mut builder = config::Config::builder()
                .set_default("data_dir", data_dir.to_str().unwrap())?
                .set_default("config_dir", config_path.to_str().unwrap())?;

            let config_files = [
                (
                    format!("{CONFIG_FILE_NAME}.json5"),
                    config::FileFormat::Json5,
                ),
                (format!("{CONFIG_FILE_NAME}.json"), config::FileFormat::Json),
                (format!("{CONFIG_FILE_NAME}.yaml"), config::FileFormat::Yaml),
                (format!("{CONFIG_FILE_NAME}.toml"), config::FileFormat::Toml),
                (format!("{CONFIG_FILE_NAME}.ini"), config::FileFormat::Ini),
            ];
            let mut found_config = false;
            for (file, format) in &config_files {
                let source = config::File::from(config_path.join(file))
                    .format(*format)
                    .required(false);
                builder = builder.add_source(source);
                if config_path.join(file).exists() {
                    found_config = true;
                }
            }
            if !found_config && !cfg!(test) {
                info!(
                    "No configuration file found.\nCreate one at {config_path:?} or generate one using `vault-tasks generate-config`"
                );
            }
            builder
        };
        let cfg: AppConfig = builder.build()?.try_deserialize()?;
        let tasks_config = TasksConfig::new(&ProtoConfig {
            vault_path: args.vault_path.clone(),
            config_path: Some(config_path),
        })
        .unwrap(); // TODO: no unwrap

        Ok(Config {
            cli: cfg,
            core: tasks_config,
        })
    }

    pub fn generate_config(path: Option<PathBuf>) -> Result<()> {
        vault_tasks_core::config::TasksConfig::generate_config(path.clone())?;
        let config_dir = path.unwrap_or_else(get_config_dir);
        let dest = config_dir.join(format!("{CONFIG_FILE_NAME}.toml"));
        if create_dir_all(config_dir).is_err() {
            bail!("Failed to create config directory at {dest:?}".to_owned());
        }
        if let Ok(mut file) = File::create(dest.clone()) {
            if file.write_all(CLI_CONFIG.as_bytes()).is_err() {
                bail!("Failed to write default config at {dest:?}".to_owned());
            }
        } else {
            bail!("Failed to create default config at {dest:?}".to_owned());
        }
        println!(
            "Configuration has been created at {}. You can fill the `vault-path` value to set a default vault.",
            dest.display()
        );
        Ok(())
    }
}
pub fn get_data_dir() -> PathBuf {
    DATA_FOLDER.clone().map_or(
        {
            project_directory().map_or_else(
                || PathBuf::from(".").join(".data"),
                |proj_dirs| proj_dirs.data_local_dir().to_path_buf(),
            )
        },
        |s| s,
    )
}

pub fn get_config_dir() -> PathBuf {
    CONFIG_FOLDER.clone().map_or_else(
        || {
            project_directory().map_or_else(
                || PathBuf::from(".").join(".config"),
                |proj_dirs| proj_dirs.config_local_dir().to_path_buf(),
            )
        },
        |s| s,
    )
}

fn project_directory() -> Option<ProjectDirs> {
    ProjectDirs::from("com", "kdheepak", env!("CARGO_PKG_NAME"))
}
