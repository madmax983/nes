use std::env;
use std::fs;
use std::path::Path;

use comfy_table::{Cell, Color as TableColor, Table};
use crossterm::style::{Color, Stylize};
use nes_core::NesCore;
use nes_mcp::macro_engine::execute_macro_script;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: nes-mcp-run-macro <rom_path> <script_path>");
        std::process::exit(1);
    }

    let rom_path = &args[1];
    let script_path = &args[2];

    if let Err(err) = run(rom_path, script_path) {
        eprintln!("\n{}", format!("Error: {err}").with(Color::Red).bold());
        std::process::exit(1);
    }
}

fn run(rom_path: &str, script_path: &str) -> Result<(), String> {
    let rom_bytes = fs::read(rom_path)
        .map_err(|err| format!("Failed to read ROM file '{}': {}", rom_path, err))?;
    let script_content = fs::read_to_string(script_path)
        .map_err(|err| format!("Failed to read script file '{}': {}", script_path, err))?;

    let mut core = NesCore::new();
    let rom_info = core
        .load_ines_rom(&rom_bytes)
        .map_err(|err| format!("Failed to load ROM: {}", err))?;

    println!("{}", "Executing Macro Script...".with(Color::Cyan).bold());

    let mut callback = |current_line: usize, total_lines: usize| {
        let pct = if total_lines > 0 {
            (current_line as f64 / total_lines as f64) * 100.0
        } else {
            0.0
        };
        print!(
            "\r\x1B[2K{}",
            format!(
                "Running line {} / {} ({:.1}%)",
                current_line, total_lines, pct
            )
            .with(Color::Yellow)
        );
        let _ = std::io::Write::flush(&mut std::io::stdout());
    };

    let frames_elapsed = execute_macro_script(&mut core, &script_content, Some(&mut callback))
        .map_err(|err| format!("Macro execution failed: {}", err))?;

    println!(
        "\r\x1B[2K{}",
        "Execution Complete".with(Color::Green).bold()
    );

    let rom_name = Path::new(rom_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(rom_path);

    let mut table = Table::new();
    table.set_header(vec![
        Cell::new("Metric").fg(TableColor::Cyan),
        Cell::new("Value").fg(TableColor::White),
    ]);

    table.add_row(vec![
        Cell::new("ROM"),
        Cell::new(rom_name).fg(TableColor::Green),
    ]);
    table.add_row(vec![
        Cell::new("Mapper ID"),
        Cell::new(rom_info.mapper_id.to_string()),
    ]);
    table.add_row(vec![
        Cell::new("Status"),
        Cell::new("Success").fg(TableColor::Green),
    ]);
    table.add_row(vec![
        Cell::new("Frames Elapsed"),
        Cell::new(frames_elapsed.to_string()).fg(TableColor::Yellow),
    ]);
    table.add_row(vec![
        Cell::new("Total CPU Cycles"),
        Cell::new(core.total_cycles().to_string()).fg(TableColor::Yellow),
    ]);
    table.add_row(vec![
        Cell::new("Final PC"),
        Cell::new(format!("${:04X}", core.cpu_pc())).fg(TableColor::Yellow),
    ]);
    table.add_row(vec![
        Cell::new("Final Controller State"),
        Cell::new(format!("{:08b}", core.controller_bits())).fg(TableColor::Yellow),
    ]);

    println!("\n{table}");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::run;

    #[test]
    fn run_reports_missing_rom_file_errors() {
        let err = run("__missing_rom__.nes", "__missing_script__.txt")
            .expect_err("missing files should fail");
        assert!(err.contains("Failed to read ROM file"));
    }
}
