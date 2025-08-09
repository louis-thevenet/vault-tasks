use color_eyre::Result;
use tracing::debug;
use tracing_error::ErrorLayer;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

use crate::config;

lazy_static::lazy_static! {
    pub static ref LOG_ENV: String = format!("{}_LOGLEVEL", config::PROJECT_NAME.clone());
    pub static ref LOG_FILE: String = format!("{}.log", env!("CARGO_PKG_NAME"));
}

pub fn init() -> Result<()> {
    let directory = config::get_data_dir();
    std::fs::create_dir_all(directory.clone())?;
    let log_path = directory.join(LOG_FILE.clone());
    let log_file = std::fs::File::create(log_path.clone())?;

    // Build the environment filter
    let env_filter = EnvFilter::builder().with_default_directive(tracing::Level::INFO.into());

    // Try to get filter from RUST_LOG first, then fallback to custom env var, then use default
    let env_filter = env_filter
        .try_from_env()
        .or_else(|_| env_filter.with_env_var(LOG_ENV.clone()).from_env())
        .unwrap_or_else(|_| {
            // If both environment variables fail, use the default INFO level
            EnvFilter::builder()
                .with_default_directive(tracing::Level::INFO.into())
                .from_env_lossy()
        });

    let file_subscriber = fmt::layer()
        .with_file(true)
        .with_line_number(true)
        .with_writer(log_file)
        .with_target(false)
        .with_ansi(false)
        .with_filter(env_filter);

    tracing_subscriber::registry()
        .with(file_subscriber)
        .with(ErrorLayer::default())
        .try_init()?;

    // Test that logging is working
    debug!("Logging initialized successfully. Log file: {:?}", log_path);
    Ok(())
}
