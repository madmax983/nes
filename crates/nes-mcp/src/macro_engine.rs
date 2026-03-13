//! A lightweight scripting engine for programmatic emulator control.
//!
//! The macro engine allows users to write simple, line-based text scripts to
//! deterministically drive the emulator's inputs and execution frames. This is
//! useful for automating repetitive tasks, creating save states at precise
//! moments, or testing specific gameplay sequences.

use nes_core::{Button, Command, NesCore};

/// Executes a simple line-based macro script on the given [`NesCore`].
///
/// Scripts are executed synchronously and block until the entire sequence
/// completes. This is useful for fast-forwarding the emulator state deterministically
/// without manual user interaction.
///
/// Supported commands:
/// - `WAIT <frames>`: Steps the emulator for the given number of frames.
/// - `PRESS <button>`: Presses the given controller button.
/// - `RELEASE <button>`: Releases the given controller button.
/// - `RESET`: Resets the emulator.
///
/// Empty lines and lines starting with `#` or `//` are ignored. Trailing text on
/// a line after a valid command is also ignored.
///
/// ## Errors
///
/// Returns an error if:
/// - A command is not recognized.
/// - A command is missing required arguments (e.g. `WAIT` without a frame count).
/// - An invalid frame count or button name is provided.
/// - The underlying [`NesCore`] fails to execute a command.
///
/// ## Examples
///
/// ```
/// use nes_core::NesCore;
/// use nes_mcp::macro_engine::execute_macro_script;
///
/// let mut core = NesCore::new();
/// // In a real scenario, you'd load a ROM first:
/// // core.load_ines_rom(&rom_bytes).unwrap();
///
/// let script = "
/// WAIT 10
/// PRESS Start
/// WAIT 1
/// RELEASE Start
/// ";
///
/// let frames_elapsed = execute_macro_script(&mut core, script).unwrap();
/// assert_eq!(frames_elapsed, 11);
/// ```
pub fn execute_macro_script(core: &mut NesCore, script: &str) -> Result<u64, String> {
    let mut frames_elapsed = 0;

    for (line_num, line) in script.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        let command_name = parts[0].to_uppercase();
        match command_name.as_str() {
            "WAIT" => {
                if parts.len() < 2 {
                    return Err(format!("Line {}: WAIT needs a frame count", line_num + 1));
                }
                let frames: u64 = parts[1].parse().map_err(|_| {
                    format!("Line {}: Invalid frame count '{}'", line_num + 1, parts[1])
                })?;
                for _ in 0..frames {
                    core.execute(Command::StepFrame)
                        .map_err(|e| format!("Line {}: Core error: {}", line_num + 1, e))?;
                    frames_elapsed += 1;
                }
            }
            "PRESS" | "HOLD" => {
                if parts.len() < 2 {
                    return Err(format!(
                        "Line {}: {} needs a button",
                        line_num + 1,
                        command_name
                    ));
                }
                let btn = parse_btn(parts[1]).ok_or_else(|| {
                    format!("Line {}: Invalid button '{}'", line_num + 1, parts[1])
                })?;
                core.execute(Command::PressButton(btn))
                    .map_err(|e| format!("Line {}: Core error: {}", line_num + 1, e))?;
            }
            "RELEASE" => {
                if parts.len() < 2 {
                    return Err(format!("Line {}: RELEASE needs a button", line_num + 1));
                }
                let btn = parse_btn(parts[1]).ok_or_else(|| {
                    format!("Line {}: Invalid button '{}'", line_num + 1, parts[1])
                })?;
                core.execute(Command::ReleaseButton(btn))
                    .map_err(|e| format!("Line {}: Core error: {}", line_num + 1, e))?;
            }
            "RESET" => {
                core.execute(Command::Reset)
                    .map_err(|e| format!("Line {}: Core error: {}", line_num + 1, e))?;
            }
            _ => {
                return Err(format!(
                    "Line {}: Unknown command '{}'",
                    line_num + 1,
                    parts[0]
                ));
            }
        }
    }

    Ok(frames_elapsed)
}

fn parse_btn(s: &str) -> Option<Button> {
    match s.to_uppercase().as_str() {
        "A" => Some(Button::A),
        "B" => Some(Button::B),
        "SELECT" => Some(Button::Select),
        "START" => Some(Button::Start),
        "UP" => Some(Button::Up),
        "DOWN" => Some(Button::Down),
        "LEFT" => Some(Button::Left),
        "RIGHT" => Some(Button::Right),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nes_core::NesCore;

    #[test]
    fn test_execute_macro_script_wait() {
        let mut core = NesCore::new();
        let initial_frames = core.ppu_frame_counter();

        let script = "WAIT 5";
        let elapsed = execute_macro_script(&mut core, script).unwrap();

        assert_eq!(elapsed, 5);
        assert_eq!(core.ppu_frame_counter(), initial_frames + 5);
    }

    #[test]
    fn test_execute_macro_script_buttons() {
        let mut core = NesCore::new();
        assert_eq!(core.controller_bits(), 0);

        let script = "
            PRESS A
            PRESS Start
        ";
        let elapsed = execute_macro_script(&mut core, script).unwrap();
        assert_eq!(elapsed, 0); // PRESS doesn't wait
        assert_eq!(
            core.controller_bits(),
            Button::A.bit_mask() | Button::Start.bit_mask()
        );

        let script2 = "RELEASE A";
        let elapsed2 = execute_macro_script(&mut core, script2).unwrap();
        assert_eq!(elapsed2, 0);
        assert_eq!(core.controller_bits(), Button::Start.bit_mask());
    }

    #[test]
    fn test_execute_macro_script_comments_and_whitespace() {
        let mut core = NesCore::new();
        let script = "
            # This is a comment
            // Also a comment

            PRESS B
            WAIT 1
            RELEASE B
        ";
        let elapsed = execute_macro_script(&mut core, script).unwrap();
        assert_eq!(elapsed, 1);
        assert_eq!(core.controller_bits(), 0);
    }

    #[test]
    fn test_execute_macro_script_invalid_command() {
        let mut core = NesCore::new();
        let script = "JUMP 10";
        let res = execute_macro_script(&mut core, script);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("Unknown command 'JUMP'"));
    }

    #[test]
    fn test_execute_macro_script_invalid_args() {
        let mut core = NesCore::new();
        let res = execute_macro_script(&mut core, "WAIT X");
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("Invalid frame count 'X'"));

        let res2 = execute_macro_script(&mut core, "PRESS MAGIC_BUTTON");
        assert!(res2.is_err());
        assert!(res2.unwrap_err().contains("Invalid button 'MAGIC_BUTTON'"));
    }

    #[test]
    fn test_execute_macro_script_accepts_trailing_tokens_for_known_commands() {
        let mut core = NesCore::new();
        let initial_frames = core.ppu_frame_counter();
        let script = "
            WAIT 2 trailing
            PRESS A trailing
            RELEASE A trailing
        ";
        let elapsed = execute_macro_script(&mut core, script).expect("script with extra args");
        assert_eq!(elapsed, 2);
        assert_eq!(core.ppu_frame_counter(), initial_frames + 2);
        assert_eq!(core.controller_bits(), 0);
    }

    #[test]
    fn test_execute_macro_script_reset_and_directional_buttons() {
        let mut core = NesCore::new();
        let script = "
            PRESS Select
            PRESS Up
            PRESS Down
            PRESS Left
            PRESS Right
            RESET
        ";
        let elapsed = execute_macro_script(&mut core, script).expect("script executes");
        assert_eq!(elapsed, 0);
        assert_eq!(core.controller_bits(), 0);
    }
}

#[cfg(test)]
mod havoc_fuzz_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(10000))]
        #[test]
        fn havoc_fuzz_macro_engine(script in "\\PC*") {
            let mut core = nes_core::NesCore::new();
            let _ = execute_macro_script(&mut core, &script);
        }
    }
}
