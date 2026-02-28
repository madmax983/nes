//! Command replay helpers.
//!
//! This module provides a tiny deterministic replay primitive used by
//! hosts, regression tests, and MCP tooling.

use crate::{Command, CoreError, NesCore};

/// Applies a command sequence to a core in-order.
///
/// # Errors
///
/// Returns the first [`CoreError`] produced while executing the stream.
pub fn replay_commands(core: &mut NesCore, commands: &[Command]) -> Result<(), CoreError> {
    for command in commands {
        core.execute(*command)?;
    }
    Ok(())
}
