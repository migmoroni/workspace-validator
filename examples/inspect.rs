//! Loads a configuration and inspects its default immutable validation plan.

use std::{error::Error, path::PathBuf};
use workspace_validator::{config, planning};

fn main() -> Result<(), Box<dyn Error>> {
    let current = std::env::current_dir()?;
    let explicit = std::env::args_os().nth(1).map(PathBuf::from);
    let validated = config::load(explicit.as_deref(), &current)?;
    let plan = planning::target(&validated, None)?;

    println!(
        "configuration: {}",
        validated.configuration_path().display()
    );
    println!("workspace: {}", validated.workspace_root().display());
    println!("selection: {:?}", plan.selection());
    println!("groups: {}", plan.groups().len());
    println!("suites: {}", plan.suites().len());
    println!("check executions: {}", plan.check_executions().len());

    Ok(())
}
