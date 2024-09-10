use anyhow::{bail, Context, Result};

use log::{debug, info};
use serde::Deserialize;
use std::{fs::read_to_string, path::PathBuf};

use crate::Args;

#[derive(Debug, Deserialize, Clone)]
struct ParsedConfig {
    ignore_dot_files: Option<bool>,
    ignored: Option<Vec<PathBuf>>,
    indent_length: Option<usize>,
    use_american_format: Option<bool>,
    vault_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub ignore_dot_files: bool,
    pub ignored: Vec<PathBuf>,
    pub indent_length: usize,
    pub use_american_format: bool,
    pub vault_path: PathBuf,
}
impl Config {
    pub fn load_config(args: &Args) -> Result<Self> {
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
        let parsed_config =
            toml::from_str::<ParsedConfig>(&content).context("Failed to parse config file")?;
        debug!("Read config: {:#?}", parsed_config);

        if !parsed_config.vault_path.exists() {
            bail!("Vault path {:?} not found.", parsed_config.vault_path);
        }
        if !parsed_config.vault_path.is_dir() {
            bail!(
                "Vault path {:?} is not a directory.",
                parsed_config.vault_path
            );
        }

        Ok(Self {
            ignore_dot_files: parsed_config.ignore_dot_files.unwrap_or(true),
            ignored: parsed_config.ignored.unwrap_or_default(),
            indent_length: parsed_config.indent_length.unwrap_or(2),
            use_american_format: parsed_config.use_american_format.unwrap_or(true),
            vault_path: parsed_config.vault_path,
        })
    }
}
impl Default for Config {
    fn default() -> Self {
        Self {
            ignore_dot_files: true,
            ignored: vec![],
            indent_length: 2,
            use_american_format: true,
            vault_path: PathBuf::new(),
        }
    }
}
