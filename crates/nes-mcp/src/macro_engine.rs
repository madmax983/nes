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
/// **Performance optimization:** Iterators are used instead of `collect::<Vec<&str>>()` to parse parts of the line.
/// This removes a heap allocation for `Vec<&str>` per non-empty line in a macro script.
/// Additionally, `eq_ignore_ascii_case` is used to match string commands and arguments without allocating strings on the heap via `.to_uppercase()`.
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
/// let frames_elapsed = execute_macro_script(&mut core, script, None).unwrap();
/// assert_eq!(frames_elapsed, 11);
/// ```
pub fn execute_macro_script(
    core: &mut NesCore,
    script: &str,
    mut progress_callback: Option<&mut dyn FnMut(usize, usize)>,
) -> Result<u64, String> {
    let mut frames_elapsed = 0;
    let total_lines = if progress_callback.is_some() {
        Some(script.lines().count())
    } else {
        None
    };

    for (line_num, line) in script.lines().enumerate() {
        if let (Some(total), Some(cb)) = (total_lines, &mut progress_callback) {
            cb(line_num + 1, total);
        }
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
            continue;
        }

        let mut parts = line.split_whitespace();
        let Some(command) = parts.next() else {
            continue;
        };

        if command.eq_ignore_ascii_case("WAIT") {
            let Some(arg) = parts.next() else {
                return Err(format!("Line {}: WAIT needs a frame count", line_num + 1));
            };
            let frames: u64 = arg
                .parse()
                .map_err(|_| format!("Line {}: Invalid frame count '{}'", line_num + 1, arg))?;
            for _ in 0..frames {
                core.execute(Command::StepFrame)
                    .map_err(|e| format!("Line {}: Core error: {}", line_num + 1, e))?;
                frames_elapsed += 1;
            }
        } else if command.eq_ignore_ascii_case("PRESS") || command.eq_ignore_ascii_case("HOLD") {
            let Some(arg) = parts.next() else {
                return Err(format!("Line {}: {} needs a button", line_num + 1, command));
            };
            let btn = parse_btn(arg)
                .ok_or_else(|| format!("Line {}: Invalid button '{}'", line_num + 1, arg))?;
            core.execute(Command::PressButton(btn))
                .map_err(|e| format!("Line {}: Core error: {}", line_num + 1, e))?;
        } else if command.eq_ignore_ascii_case("RELEASE") {
            let Some(arg) = parts.next() else {
                return Err(format!("Line {}: RELEASE needs a button", line_num + 1));
            };
            let btn = parse_btn(arg)
                .ok_or_else(|| format!("Line {}: Invalid button '{}'", line_num + 1, arg))?;
            core.execute(Command::ReleaseButton(btn))
                .map_err(|e| format!("Line {}: Core error: {}", line_num + 1, e))?;
        } else if command.eq_ignore_ascii_case("RESET") {
            core.execute(Command::Reset)
                .map_err(|e| format!("Line {}: Core error: {}", line_num + 1, e))?;
        } else {
            return Err(format!(
                "Line {}: Unknown command '{}'",
                line_num + 1,
                command
            ));
        }
    }

    Ok(frames_elapsed)
}

#[inline]
fn parse_btn(s: &str) -> Option<Button> {
    if s.eq_ignore_ascii_case("A") {
        Some(Button::A)
    } else if s.eq_ignore_ascii_case("B") {
        Some(Button::B)
    } else if s.eq_ignore_ascii_case("SELECT") {
        Some(Button::Select)
    } else if s.eq_ignore_ascii_case("START") {
        Some(Button::Start)
    } else if s.eq_ignore_ascii_case("UP") {
        Some(Button::Up)
    } else if s.eq_ignore_ascii_case("DOWN") {
        Some(Button::Down)
    } else if s.eq_ignore_ascii_case("LEFT") {
        Some(Button::Left)
    } else if s.eq_ignore_ascii_case("RIGHT") {
        Some(Button::Right)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nes_core::NesCore;

    #[test]
    fn test_execute_macro_script_wait() -> Result<(), Box<dyn std::error::Error>> {
        let mut core = NesCore::new();
        let initial_frames = core.ppu_frame_counter();

        let script = "WAIT 5";
        let elapsed = execute_macro_script(&mut core, script, None)?;

        assert_eq!(elapsed, 5);
        assert_eq!(core.ppu_frame_counter(), initial_frames + 5);
        Ok(())
    }

    #[test]
    fn test_execute_macro_script_with_progress_callback() -> Result<(), Box<dyn std::error::Error>>
    {
        let mut core = NesCore::new();
        let script = "
            WAIT 1
            PRESS A
            RELEASE A
        ";

        let mut lines_reported = 0;
        let mut max_total_lines = 0;
        let mut callback = |current: usize, total: usize| {
            lines_reported += 1;
            max_total_lines = total;
            // The script has 5 lines total (including empty lines).
            assert!(current <= total);
        };

        execute_macro_script(&mut core, script, Some(&mut callback))?;

        assert_eq!(lines_reported, 5);
        assert_eq!(max_total_lines, 5);
        Ok(())
    }

    #[test]
    fn test_execute_macro_script_buttons() -> Result<(), Box<dyn std::error::Error>> {
        let mut core = NesCore::new();
        assert_eq!(core.controller_bits(), 0);

        let script = "
            PRESS A
            PRESS Start
        ";
        let elapsed = execute_macro_script(&mut core, script, None)?;
        assert_eq!(elapsed, 0); // PRESS doesn't wait
        assert_eq!(
            core.controller_bits(),
            Button::A.bit_mask() | Button::Start.bit_mask()
        );

        let script2 = "RELEASE A";
        let elapsed2 = execute_macro_script(&mut core, script2, None)?;
        assert_eq!(elapsed2, 0);
        assert_eq!(core.controller_bits(), Button::Start.bit_mask());
        Ok(())
    }

    #[test]
    fn test_execute_macro_script_comments_and_whitespace() -> Result<(), Box<dyn std::error::Error>>
    {
        let mut core = NesCore::new();
        let script = "
            # This is a comment
            // Also a comment

            PRESS B
            WAIT 1
            RELEASE B
        ";
        let elapsed = execute_macro_script(&mut core, script, None)?;
        assert_eq!(elapsed, 1);
        assert_eq!(core.controller_bits(), 0);
        Ok(())
    }

    #[test]
    fn test_execute_macro_script_invalid_command() {
        let mut core = NesCore::new();
        let script = "JUMP 10";
        let res = execute_macro_script(&mut core, script, None);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("Unknown command 'JUMP'"));
    }

    #[test]
    fn test_execute_macro_script_invalid_args() {
        let mut core = NesCore::new();
        let res = execute_macro_script(&mut core, "WAIT X", None);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("Invalid frame count 'X'"));

        let res2 = execute_macro_script(&mut core, "PRESS MAGIC_BUTTON", None);
        assert!(res2.is_err());
        assert!(res2.unwrap_err().contains("Invalid button 'MAGIC_BUTTON'"));
    }

    #[test]
    fn test_execute_macro_script_accepts_trailing_tokens_for_known_commands()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut core = NesCore::new();
        let initial_frames = core.ppu_frame_counter();
        let script = "
            WAIT 2 trailing
            PRESS A trailing
            RELEASE A trailing
        ";
        let elapsed = execute_macro_script(&mut core, script, None)?;
        assert_eq!(elapsed, 2);
        assert_eq!(core.ppu_frame_counter(), initial_frames + 2);
        assert_eq!(core.controller_bits(), 0);
        Ok(())
    }

    #[test]
    fn test_execute_macro_script_reset_and_directional_buttons()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut core = NesCore::new();
        let script = "
            PRESS Select
            PRESS Up
            PRESS Down
            PRESS Left
            PRESS Right
            RESET
        ";
        let elapsed = execute_macro_script(&mut core, script, None)?;
        assert_eq!(elapsed, 0);
        assert_eq!(core.controller_bits(), 0);
        Ok(())
    }

    #[test]
    fn test_execute_macro_script_hold_alias_and_lowercase_buttons()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut core = NesCore::new();
        assert_eq!(core.controller_bits(), 0);

        let script = "
            HOLD a
            HOLD start
        ";
        let elapsed = execute_macro_script(&mut core, script, None)?;
        assert_eq!(elapsed, 0);
        assert_eq!(
            core.controller_bits(),
            Button::A.bit_mask() | Button::Start.bit_mask()
        );

        let script2 = "RELEASE a";
        let elapsed2 = execute_macro_script(&mut core, script2, None)?;
        assert_eq!(elapsed2, 0);
        assert_eq!(core.controller_bits(), Button::Start.bit_mask());
        Ok(())
    }

    #[test]
    fn test_execute_macro_script_missing_arguments() {
        let mut core = NesCore::new();

        let wait_err = execute_macro_script(&mut core, "WAIT", None).unwrap_err();
        assert!(
            wait_err.contains("WAIT needs a frame count"),
            "Expected missing frame count error for WAIT"
        );

        let press_err = execute_macro_script(&mut core, "PRESS", None).unwrap_err();
        assert!(
            press_err.contains("PRESS needs a button"),
            "Expected missing button error for PRESS"
        );

        let hold_err = execute_macro_script(&mut core, "HOLD", None).unwrap_err();
        assert!(
            hold_err.contains("HOLD needs a button"),
            "Expected missing button error for HOLD"
        );

        let release_err = execute_macro_script(&mut core, "RELEASE", None).unwrap_err();
        assert!(
            release_err.contains("RELEASE needs a button"),
            "Expected missing button error for RELEASE"
        );
    }
}
