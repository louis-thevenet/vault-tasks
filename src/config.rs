use anyhow::{bail, Context, Result};

use log::{debug, info};
use serde::Deserialize;
use std::{fs::read_to_string, path::PathBuf};

use crate::Args;

#[derive(Debug, Deserialize, Default)]
pub struct Config {
    pub vault_path: PathBuf,
    /// Default is true
    pub use_american_format: Option<bool>,
}
impl Config {
    pub fn load_config(args: &Args) -> Result<Config> {
        let config_path = if let Some(path) = args.config_path.clone() {
            path
        } else if let Some(user_dirs) = directories::UserDirs::new() {
            user_dirs.home_dir().to_path_buf()
        } else {
            bail!("Could not find config path");
        };
        let config_file = config_path.join(".vault-tasks.toml");
        info!("Loading config from {:?}", config_file);

        let content = read_to_string(&config_file).context("Failed to read config file")?;
        let config = toml::from_str::<Config>(&content).context("Failed to parse config file")?;
        debug!("{:#?}", config);

        if !config.vault_path.exists() {
            bail!("Vault path {:?} not found.", config.vault_path);
        }
        if !config.vault_path.is_dir() {
            bail!("Vault path {:?} is not a directory.", config.vault_path);
        }
        Ok(config)
    }
}
