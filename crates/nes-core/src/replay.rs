use crate::{Command, CoreError, NesCore};

pub fn replay_commands(core: &mut NesCore, commands: &[Command]) -> Result<(), CoreError> {
    for command in commands {
        core.execute(*command)?;
    }
    Ok(())
}
