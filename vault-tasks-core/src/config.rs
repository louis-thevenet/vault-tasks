#![allow(dead_code)]
use std::{
    env,
    fs::{File, create_dir_all},
    io::Write,
    path::PathBuf,
};

use color_eyre::{Result, eyre::bail};
use directories::ProjectDirs;
use lazy_static::lazy_static;
use serde::Deserialize;
use tracing::{debug, info};

const CONFIG: &str = include_str!("../../.config/core.toml");
const CONFIG_FILE_NAME: &str = "core";

lazy_static! {
    pub(crate) static ref PROJECT_NAME: String = env!("CARGO_CRATE_NAME").to_uppercase();
    pub(crate) static ref DATA_FOLDER: Option<PathBuf> =
        env::var(format!("{}_DATA", PROJECT_NAME.clone()))
            .ok()
            .map(PathBuf::from);
    pub(crate) static ref CONFIG_FOLDER: Option<PathBuf> =
        env::var(format!("{}_CONFIG", PROJECT_NAME.clone()))
            .ok()
            .map(PathBuf::from);
}

/// Characters used to mark the state of a task in Markdown.
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct TaskMarkerConfig {
    pub done: char,
    pub todo: char,
    pub incomplete: char,
    pub canceled: char,
}

// Mostly for tests
impl Default for TaskMarkerConfig {
    fn default() -> Self {
        Self {
            done: 'x',
            todo: ' ',
            incomplete: '/',
            canceled: '-',
        }
    }
}

/// Configuration for pretty symbols in the UI.
#[derive(Clone, Debug, Deserialize, Default)]
pub struct PrettySymbolsConfig {
    #[serde(default)]
    pub(crate) task_done: String,
    #[serde(default)]
    pub(crate) task_todo: String,
    #[serde(default)]
    pub(crate) task_incomplete: String,
    #[serde(default)]
    pub(crate) task_canceled: String,
    #[serde(default)]
    pub(crate) due_date: String,
    #[serde(default)]
    pub(crate) priority: String,
    #[serde(default)]
    pub(crate) today_tag: String,
    #[serde(default)]
    pub(crate) progress_bar_true: String,
    #[serde(default)]
    pub(crate) progress_bar_false: String,
}
/// Configuration for the core features of the application.
#[derive(Clone, Debug, Deserialize, Default)]
pub struct CoreConfig {
    #[serde(default)]
    pub vault_paths: Vec<PathBuf>,
    #[serde(default)]
    pub use_american_format: bool,
    #[serde(default)]
    pub(crate) parse_dot_files: bool,
    #[serde(default)]
    pub(crate) file_tags_propagation: bool,
    #[serde(default)]
    pub(crate) ignored: Vec<PathBuf>,
    #[serde(default)]
    pub(crate) indent_length: usize,
    #[serde(default)]
    pub(crate) tasks_drop_file: String,
    #[serde(default)]
    pub(crate) tracker_extra_blanks: usize,
}
#[derive(Clone, Debug, Deserialize, Default)]
pub struct DisplayConfig {
    #[serde(default)]
    pub show_relative_due_dates: bool,
}
#[derive(Clone, Debug, Deserialize)]
pub struct TasksConfig {
    #[serde(default)]
    pub core: CoreConfig,
    #[serde(default)]
    pub display: DisplayConfig,
    #[serde(default)]
    pub pretty_symbols: PrettySymbolsConfig,
    #[serde(default)]
    pub(crate) task_state_markers: TaskMarkerConfig,
}

impl Default for TasksConfig {
    fn default() -> Self {
        let mut config: Self = toml::from_str(CONFIG).unwrap();
        if cfg!(test) {
            config.core.vault_paths = vec![PathBuf::from("./test-vault")];
        }
        config
    }
}
pub struct ProtoConfig {
    pub vault_paths: Vec<PathBuf>,
    pub config_path: Option<PathBuf>,
}
impl TasksConfig {
    pub fn new(params: &ProtoConfig) -> Result<Self> {
        let default_config: Self = Self::default();
        let data_dir = get_data_dir();
        let config_path = params.config_path.clone().unwrap_or_else(get_config_dir);
        debug!(
            "Using data directory at {} and config directory at {}",
            data_dir.display(),
            config_path.display()
        );

        // A config file was provided
        let builder = if config_path.is_file() {
            config::Config::builder()
                .set_default("data_dir", data_dir.to_str().unwrap())?
                .add_source(config::File::from(config_path))
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
            // TODO: clean up this error
            if !found_config && !cfg!(test) {
                info!(
                    "No configuration file found.\nCreate one at {config_path:?} or generate one using `vault-tasks generate-config`"
                );
            }
            builder
        };

        let mut cfg: Self = builder.build()?.try_deserialize()?;

        cfg = Self::merge_tasks_config(cfg, default_config);

        cfg.core.vault_paths.append(params.vault_paths);

        Ok(cfg)
    }

    fn merge_tasks_config(user_config: TasksConfig, default_config: TasksConfig) -> TasksConfig {
        TasksConfig {
            core: CoreConfig {
                parse_dot_files: user_config.core.parse_dot_files,
                file_tags_propagation: user_config.core.file_tags_propagation,
                ignored: if user_config.core.ignored.is_empty() {
                    default_config.core.ignored
                } else {
                    user_config.core.ignored
                },
                indent_length: if user_config.core.indent_length == 0 {
                    default_config.core.indent_length
                } else {
                    user_config.core.indent_length
                },
                use_american_format: user_config.core.use_american_format,

                vault_paths: if user_config.core.vault_paths.is_empty() {
                    default_config.core.vault_paths
                } else {
                    user_config.core.vault_paths
                },
                tasks_drop_file: if user_config.core.tasks_drop_file.is_empty() {
                    default_config.core.tasks_drop_file
                } else {
                    user_config.core.tasks_drop_file
                },
                tracker_extra_blanks: if user_config.core.tracker_extra_blanks == 0 {
                    default_config.core.tracker_extra_blanks
                } else {
                    user_config.core.tracker_extra_blanks
                },
            },

            display: DisplayConfig {
                show_relative_due_dates: user_config.display.show_relative_due_dates,
            },
            task_state_markers: user_config.task_state_markers,
            pretty_symbols: Self::merge_pretty_symbols_config(
                user_config.pretty_symbols,
                default_config.pretty_symbols,
            ),
        }
    }

    fn merge_pretty_symbols_config(
        user_config: PrettySymbolsConfig,
        default_config: PrettySymbolsConfig,
    ) -> PrettySymbolsConfig {
        PrettySymbolsConfig {
            task_done: if user_config.task_done.is_empty() {
                default_config.task_done
            } else {
                user_config.task_done
            },
            task_todo: if user_config.task_todo.is_empty() {
                default_config.task_todo
            } else {
                user_config.task_todo
            },
            task_incomplete: if user_config.task_incomplete.is_empty() {
                default_config.task_incomplete
            } else {
                user_config.task_incomplete
            },
            task_canceled: if user_config.task_canceled.is_empty() {
                default_config.task_canceled
            } else {
                user_config.task_canceled
            },
            due_date: if user_config.due_date.is_empty() {
                default_config.due_date
            } else {
                user_config.due_date
            },
            priority: if user_config.priority.is_empty() {
                default_config.priority
            } else {
                user_config.priority
            },
            today_tag: if user_config.today_tag.is_empty() {
                default_config.today_tag
            } else {
                user_config.today_tag
            },
            progress_bar_true: if user_config.progress_bar_true.is_empty() {
                default_config.progress_bar_true
            } else {
                user_config.progress_bar_true
            },
            progress_bar_false: if user_config.progress_bar_false.is_empty() {
                default_config.progress_bar_false
            } else {
                user_config.progress_bar_false
            },
        }
    }

    pub fn generate_config(path: Option<PathBuf>) -> Result<()> {
        let config_dir = path.unwrap_or_else(get_config_dir);
        let dest = config_dir.join(format!("{CONFIG_FILE_NAME}.toml"));
        if create_dir_all(config_dir).is_err() {
            bail!("Failed to create config directory at {dest:?}".to_owned());
        }
        if let Ok(mut file) = File::create(dest.clone()) {
            if file.write_all(CONFIG.as_bytes()).is_err() {
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

pub(crate) fn get_data_dir() -> PathBuf {
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

pub(crate) fn get_config_dir() -> PathBuf {
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

// TODO: change this
fn project_directory() -> Option<ProjectDirs> {
    ProjectDirs::from("com", "kdheepak", env!("CARGO_PKG_NAME"))
}
