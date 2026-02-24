use std::env;
use std::fs;
use std::io::{self, BufRead};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use nes_core::{Command, NesCore};
use nes_desktop::app::map_key_event_to_command;

const TARGET_FRAME_TIME: Duration = Duration::from_millis(16);
const DEFAULT_CPU_STEPS_PER_FRAME: u32 = 10_000;

fn main() {
    let rom_path = match resolve_rom_path() {
        Ok(path) => path,
        Err(msg) => {
            eprintln!("{msg}");
            return;
        }
    };

    let rom_bytes = match fs::read(&rom_path) {
        Ok(bytes) => bytes,
        Err(err) => {
            eprintln!("Failed to read ROM at '{rom_path}': {err}");
            return;
        }
    };

    let mut core = NesCore::new();
    let info = match core.load_ines_rom(&rom_bytes) {
        Ok(info) => info,
        Err(err) => {
            eprintln!("Failed to load ROM: {err}");
            return;
        }
    };

    let steps_per_frame = env::var("NES_CPU_STEPS_PER_FRAME")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(DEFAULT_CPU_STEPS_PER_FRAME);

    println!("Loaded ROM: {rom_path}");
    println!(
        "Mapper {}, PRG {} bytes, reset vector ${:04X}",
        info.mapper_id, info.prg_rom_bytes, info.reset_pc
    );
    println!("Runtime controls:");
    println!("  press <z|x|start|select|up|down|left|right>");
    println!("  release <z|x|start|select|up|down|left|right>");
    println!("  tap <z|x|start|select|up|down|left|right>");
    println!("  quit");

    let (tx, rx) = mpsc::channel::<String>();
    thread::spawn(move || {
        let stdin = io::stdin();
        for line in stdin.lock().lines().map_while(Result::ok) {
            if tx.send(line).is_err() {
                break;
            }
        }
    });

    let mut frame_index: u64 = 0;
    let mut running = true;
    while running {
        let frame_start = Instant::now();

        while let Ok(line) = rx.try_recv() {
            if !apply_console_command(&mut core, &line) {
                running = false;
                break;
            }
        }

        if !running {
            break;
        }

        for _ in 0..steps_per_frame {
            if let Err(err) = core.execute(Command::StepCpu) {
                eprintln!("CPU halted at PC ${:04X}: {err}", core.cpu_pc());
                running = false;
                break;
            }
        }

        frame_index = frame_index.saturating_add(1);
        if frame_index.is_multiple_of(30) {
            let regs = core.cpu_snapshot();
            println!(
                "frame={} ppu_frame={} pc=${:04X} a={:02X} x={:02X} y={:02X} ctrl={:02X}",
                frame_index,
                core.ppu_frame_counter(),
                regs.pc,
                regs.a,
                regs.x,
                regs.y,
                core.controller_bits()
            );
        }

        let elapsed = frame_start.elapsed();
        if elapsed < TARGET_FRAME_TIME {
            thread::sleep(TARGET_FRAME_TIME - elapsed);
        }
    }
}

fn resolve_rom_path() -> Result<String, String> {
    if let Some(path) = env::args().nth(1) {
        return Ok(path);
    }
    if let Ok(path) = env::var("SMB_ROM_PATH")
        && !path.trim().is_empty()
    {
        return Ok(path);
    }
    Err("Provide ROM path as first argument or set SMB_ROM_PATH.".to_owned())
}

fn apply_console_command(core: &mut NesCore, line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.eq_ignore_ascii_case("quit") || trimmed.eq_ignore_ascii_case("q") {
        return false;
    }

    let mut parts = trimmed.split_whitespace();
    let Some(cmd) = parts.next() else {
        return true;
    };
    let Some(button_name) = parts.next() else {
        eprintln!("Command requires a button: {trimmed}");
        return true;
    };

    let Some(key_code) = button_to_keycode(button_name) else {
        eprintln!("Unknown button '{button_name}'");
        return true;
    };

    match cmd {
        "press" => {
            if let Some(mapped) = map_key_event_to_command(key_code, true) {
                let _ = core.execute(mapped.core);
            }
        }
        "release" => {
            if let Some(mapped) = map_key_event_to_command(key_code, false) {
                let _ = core.execute(mapped.core);
            }
        }
        "tap" => {
            if let Some(mapped) = map_key_event_to_command(key_code, true) {
                let _ = core.execute(mapped.core);
            }
            if let Some(mapped) = map_key_event_to_command(key_code, false) {
                let _ = core.execute(mapped.core);
            }
        }
        _ => {
            eprintln!("Unknown command '{cmd}'");
        }
    }

    true
}

fn button_to_keycode(name: &str) -> Option<&'static str> {
    match name.to_ascii_lowercase().as_str() {
        "z" | "a" => Some("KeyZ"),
        "x" | "b" => Some("KeyX"),
        "start" => Some("Enter"),
        "select" => Some("ShiftRight"),
        "up" => Some("ArrowUp"),
        "down" => Some("ArrowDown"),
        "left" => Some("ArrowLeft"),
        "right" => Some("ArrowRight"),
        _ => None,
    }
}
