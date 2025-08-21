use std::path::PathBuf;

use clap::{ArgGroup, Args, Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

/// Manage tasks from your Markdown vault.
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// List tasks
    List {
        #[command(flatten)]
        file_selector_args: FileSelectorArgs,
        #[command(flatten)]
        filter_args: TaskFilterArgs,
    },
    /// Add a new task
    Add {
        /// Task description using regular vault-tasks syntax: "- [ ] example task tomorrow #tag"
        // #[arg(short, long, allow_hyphen_values = true)]
        #[arg(allow_hyphen_values = true)]
        task: String,
        #[command(flatten)]
        args: FileSelectorArgs,
    },
    /// Change a task's state
    Mark {
        /// Task description
        new_state: CliTaskState,
        #[command(flatten)]
        file_selector_args: FileSelectorArgs,
        #[command(flatten)]
        filter_args: TaskFilterArgs,
    },
}

/// Filter tasks based on different criteria.
#[derive(Args, Debug)]
pub struct TaskFilterArgs {
    /// Number of elements to return.
    #[arg(short, long, alias = "n")]
    pub limit: Option<usize>,
    /// Tags to keep separated by comma.
    #[arg(long, alias = "wt", value_delimiter = ',')]
    pub with_tag: Vec<String>,
    /// Tags to exclude separated by comma.
    #[arg(long, alias = "nt", value_delimiter = ',')]
    pub without_tag: Vec<String>,
    /// States to keep separated by comma.
    #[arg(long, alias = "ws", value_delimiter = ',', default_value = "todo")]
    pub with_state: Vec<CliTaskState>,
    /// States to exclude separated by comma.
    #[arg(long, alias = "ns", value_delimiter = ',')]
    pub without_state: Vec<CliTaskState>,
}

#[derive(Args, Debug)]
#[command(group(
    ArgGroup::new("add")
        .args(["path", "fuzzy"]),
))]
pub struct FileSelectorArgs {
    /// Select paths with fuzzy finding.
    #[arg(long)]
    fuzzy: bool,

    /// Path(s) to act on. Some command will fail if more than one path is provided.
    ///
    /// If no paths are provided, the default path from config file will be used.
    /// If no default path is set, current working directory will be used.
    // #[arg(short, long)]
    path: Vec<PathBuf>,
}

/// Possible states for a task.
#[derive(Debug, Clone, PartialEq, Eq, ValueEnum)]
pub enum CliTaskState {
    Done,
    Todo,
    Cancelled,
    Incomplete,
    All,
}
