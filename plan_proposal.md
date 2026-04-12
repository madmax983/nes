1. **Apply `Mosaic` persona philosophy**: Identify inconsistent and unformatted error output in the `nes-tui` CLI application. We will wrap the raw `eprintln!("{err}")` in `nes-tui/src/main.rs` with `crossterm::style::Stylize` to make the error messages pop and match the aesthetic of `nes-desktop`.
   - Update `crates/nes-tui/src/main.rs` to format the main function's error output: `eprintln!("\n{}", format!("Error: {err}").with(crossterm::style::Color::Red).bold());`.
2. **Review other raw error printouts in `nes-tui`**: Ensure all top-level exit error printouts use formatted styling.
3. **Execute Pre-commit Checks**: Run tests and format.
   - Run `cargo fmt --all`.
   - Run `cargo test -p nes-tui`.
   - Run `cargo clippy --all-targets --all-features -- -D warnings`.
4. **Create PR output**: Document the changes as required by the `Mosaic` persona (Title: "🎨 Mosaic: UI Polish for [Target Module]", Before, After, Visuals).
