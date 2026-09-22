//! Declarative, shell-free validation graphs for heterogeneous workspaces.
//!
//! The validation pipeline is deliberately split into four stages:
//! [`config`] loads and validates a document, [`planning`] creates an immutable
//! execution closure, [`execution`] runs it, and [`reporting`] renders the
//! resulting versioned report. The `workspace-validator` binary composes the
//! same public library API exposed by this crate.
//!
//! Configuration is trusted input. Commands run without an intermediate
//! shell, but declared programs still execute with the permissions of the
//! current process.
//!
//! # Example
//!
//! ```no_run
//! use std::{
//!     path::Path,
//!     sync::{atomic::AtomicBool, Arc},
//! };
//! use workspace_validator::{config, execution, planning};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let current = std::env::current_dir()?;
//! let validated = config::load(Some(Path::new(".validation/config.json")), &current)?;
//! let plan = planning::target(&validated, None)?;
//! let outcome = execution::run(&validated, &plan, Arc::new(AtomicBool::new(false)));
//! println!("{:?}", outcome.report.summary.result);
//! # Ok(())
//! # }
//! ```

#![warn(missing_docs)]
#![deny(rustdoc::bare_urls)]
#![deny(rustdoc::broken_intra_doc_links)]

mod cli;
mod dag;
mod error;
mod process;

pub mod config;
pub mod contracts;
pub mod execution;
pub mod planning;
pub mod reporting;
mod repository;

pub use cli::run_cli;
pub use error::ValidatorError;
