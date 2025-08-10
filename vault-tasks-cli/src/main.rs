mod errors;

use color_eyre::Result;
use vault_tasks_core::init_logging;

fn main() -> Result<()> {
    crate::errors::init()?;
    init_logging()?;

    println!("Hello, world!");
    Ok(())
}
