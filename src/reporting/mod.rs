//! Presentation boundaries for validation execution and completed results.
//!
//! Active runs publish transient progress through an internal terminal
//! renderer. Immutable reports are rendered only after completion through
//! [`result`]. Shared text formatting remains private to this module tree.

pub(crate) mod execution;
/// Rendering of an immutable completed validation report.
pub mod result;
/// Composable palettes and presentations shared by human reporting stages.
pub mod theme;

mod format;
mod tree;
