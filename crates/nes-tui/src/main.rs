use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use nes_config::{DEFAULT_CONFIG_PATH, NesConfig, parse_config_path_arg};
use nes_core::{Command, FRAME_RGBA_BYTES, NesCore};
use nes_tui::app::map_key_event_to_command;
use nes_tui::render::{frame_lines_half_blocks, mini_palette_spans};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

const TARGET_FRAME_TIME: Duration = Duration::from_micros(16_667);

struct TuiRuntime {
    core: NesCore,
    rom_name: String,
    mapper_id: u8,
    prg_rom_bytes: usize,
    frame_rgba: Vec<u8>,
    paused: bool,
    frames_rendered: u64,
    started_at: Instant,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
    }
}

fn run() -> Result<(), String> {
    let (rom_path, loaded_config_path) = resolve_rom_path()?;
    let rom_bytes =
        fs::read(&rom_path).map_err(|err| format!("Failed to read ROM at '{rom_path}': {err}"))?;

    let mut core = NesCore::new();
    let info = core
        .load_ines_rom(&rom_bytes)
        .map_err(|err| format!("Failed to load ROM: {err}"))?;
    let rom_name = Path::new(&rom_path)
        .file_name()
        .and_then(|name| name.to_str())
        .map(std::borrow::ToOwned::to_owned)
        .unwrap_or(rom_path.clone());
    let mut runtime = TuiRuntime {
        core,
        rom_name,
        mapper_id: info.mapper_id,
        prg_rom_bytes: info.prg_rom_bytes,
        frame_rgba: vec![0_u8; FRAME_RGBA_BYTES],
        paused: false,
        frames_rendered: 0,
        started_at: Instant::now(),
    };
    if let Some(config_path) = loaded_config_path.as_ref() {
        eprintln!("Config: {}", config_path.display());
    }

    let mut stdout = io::stdout();
    enable_raw_mode().map_err(|err| format!("Failed to enable raw mode: {err}"))?;
    execute!(stdout, EnterAlternateScreen)
        .map_err(|err| format!("Failed to enter alternate screen: {err}"))?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal =
        Terminal::new(backend).map_err(|err| format!("Failed to initialize terminal: {err}"))?;
    terminal
        .clear()
        .map_err(|err| format!("Failed to clear terminal: {err}"))?;

    let loop_result = event_loop(&mut terminal, &mut runtime);

    disable_raw_mode().ok();
    execute!(terminal.backend_mut(), LeaveAlternateScreen).ok();
    terminal.show_cursor().ok();

    loop_result
}

fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    runtime: &mut TuiRuntime,
) -> Result<(), String> {
    let mut next_frame_deadline = Instant::now();

    loop {
        while event::poll(Duration::from_millis(0))
            .map_err(|err| format!("Input poll failed: {err}"))?
        {
            let ev = event::read().map_err(|err| format!("Input read failed: {err}"))?;
            if let Event::Key(key) = ev {
                if should_quit(key) {
                    return Ok(());
                }

                if key_is_pressed(key.kind) && key.code == KeyCode::Char('p') {
                    runtime.paused = !runtime.paused;
                    let cmd = if runtime.paused {
                        Command::Pause
                    } else {
                        Command::Resume
                    };
                    runtime
                        .core
                        .execute(cmd)
                        .map_err(|err| format!("Pause toggle failed: {err}"))?;
                } else if key_is_pressed(key.kind) && key.code == KeyCode::Char('r') {
                    runtime
                        .core
                        .execute(Command::Reset)
                        .map_err(|err| format!("Reset failed: {err}"))?;
                }

                if let Some(pressed) = key_pressed_state(key.kind)
                    && let Some(mapped) = map_key_event_to_command(key.code, pressed)
                {
                    runtime
                        .core
                        .execute(mapped.core)
                        .map_err(|err| format!("Controller input failed: {err}"))?;
                }
            }
        }

        let now = Instant::now();
        if now < next_frame_deadline {
            std::thread::sleep(Duration::from_millis(1));
            continue;
        }
        next_frame_deadline = now + TARGET_FRAME_TIME;

        if !runtime.paused {
            runtime.core.execute(Command::StepFrame).map_err(|err| {
                format!(
                    "Frame step failed at PC ${:04X}: {err}",
                    runtime.core.cpu_pc()
                )
            })?;
            runtime.frames_rendered = runtime.frames_rendered.saturating_add(1);
        }

        runtime.core.fill_framebuffer_rgba(&mut runtime.frame_rgba);
        draw_frame(terminal, runtime)?;
    }
}

fn draw_frame(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    runtime: &TuiRuntime,
) -> Result<(), String> {
    terminal
        .draw(|f| {
            let root = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(2),
                    Constraint::Min(10),
                    Constraint::Length(8),
                    Constraint::Length(1),
                ])
                .split(f.area());

            let regs = runtime.core.cpu_snapshot();
            let status = if runtime.paused { "paused" } else { "running" };
            let elapsed = runtime.started_at.elapsed().as_secs_f64().max(1e-6);
            let wall_fps = runtime.frames_rendered as f64 / elapsed;
            let header = Paragraph::new(vec![
                Line::styled(
                    "NES-TUI | truecolor half-block renderer",
                    Style::default().fg(Color::Cyan),
                ),
                Line::styled(
                    format!(
                        "{} | mapper {} | frame {} | wall_fps {:.1} | {status}",
                        runtime.rom_name,
                        runtime.mapper_id,
                        runtime.frames_rendered,
                        wall_fps,
                    ),
                    Style::default().fg(Color::White),
                ),
            ]);
            f.render_widget(header, root[0]);

            let video_area = root[1];
            let panel_area = root[2];

            let video_block = Block::default()
                .title(Span::styled(" Screen ", Style::default().fg(Color::Green)))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray));
            let video_inner = video_block.inner(video_area);
            f.render_widget(video_block, video_area);

            if video_inner.width == 0 || video_inner.height == 0 {
                return;
            }

            let video_lines =
                frame_lines_half_blocks(&runtime.frame_rgba, video_inner.width, video_inner.height);
            f.render_widget(Paragraph::new(video_lines), video_inner);

            let panels = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(62), Constraint::Percentage(38)])
                .split(panel_area);

            let stats = vec![
                Line::styled(
                    format!("ROM: {}", runtime.rom_name),
                    Style::default().fg(Color::White),
                ),
                Line::styled(
                    format!(
                        "PRG: {} KiB  mapper: {}",
                        runtime.prg_rom_bytes / 1024,
                        runtime.mapper_id
                    ),
                    Style::default().fg(Color::Gray),
                ),
                Line::styled(
                    format!(
                        "CPU: PC ${:04X}  A {:02X}  X {:02X}  Y {:02X}",
                        regs.pc, regs.a, regs.x, regs.y
                    ),
                    Style::default().fg(Color::Yellow),
                ),
                Line::styled(
                    format!(
                        "PPU: frame {}  sl {}  dot {}",
                        runtime.core.ppu_frame_counter(),
                        runtime.core.ppu_scanline(),
                        runtime.core.ppu_dot()
                    ),
                    Style::default().fg(Color::LightBlue),
                ),
                Line::styled(
                    format!("State: {status}"),
                    Style::default().fg(if runtime.paused {
                        Color::LightRed
                    } else {
                        Color::LightGreen
                    }),
                ),
                Line::from({
                    let mut spans = vec![Span::styled(
                        "Palette: ",
                        Style::default().fg(Color::DarkGray),
                    )];
                    spans.extend(mini_palette_spans(&runtime.frame_rgba, 10));
                    spans
                }),
            ];
            let stats_widget = Paragraph::new(stats).block(
                Block::default()
                    .title(Span::styled(" HUD ", Style::default().fg(Color::Magenta)))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)),
            );
            f.render_widget(stats_widget, panels[0]);

            let controls = vec![
                Line::styled(
                    "Arrows=DPAD  Z=A  X=B",
                    Style::default().fg(Color::Gray),
                ),
                Line::styled(
                    "Enter=Start  Tab/C=Select",
                    Style::default().fg(Color::Gray),
                ),
                Line::styled(
                    "P=Pause  R=Reset  Q/Esc=Quit",
                    Style::default().fg(Color::Gray),
                ),
            ];
            let controls_widget = Paragraph::new(controls).block(
                Block::default()
                    .title(Span::styled(" Controls ", Style::default().fg(Color::Cyan)))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)),
            );
            f.render_widget(controls_widget, panels[1]);

            let footer = Paragraph::new(Line::styled(
                "Tip: maximize terminal for denser image; this renderer uses upper-half blocks for 2x vertical detail.",
                Style::default().fg(Color::DarkGray),
            ));
            f.render_widget(footer, root[2]);
        })
        .map(|_| ())
        .map_err(|err| format!("Render failed: {err}"))
}

fn resolve_rom_path() -> Result<(String, Option<PathBuf>), String> {
    let raw_args: Vec<String> = std::env::args().skip(1).collect();
    let (config_path, pass_through) = parse_config_path_arg(&raw_args)?;

    let mut rom_path_arg = None::<String>;
    for arg in pass_through {
        if arg == "--help" || arg == "-h" {
            return Err(format!(
                "Usage: nes-tui [--config <path>] [rom_path]\nDefault config path: {DEFAULT_CONFIG_PATH}"
            ));
        }
        if arg.starts_with("--") {
            return Err(format!(
                "unknown flag '{arg}'. Usage: nes-tui [--config <path>] [rom_path]"
            ));
        }
        if rom_path_arg.is_some() {
            return Err(
                "multiple ROM paths provided; expected at most one positional ROM path".to_owned(),
            );
        }
        rom_path_arg = Some(arg);
    }

    let loaded_config_path = config_path.clone().or_else(|| {
        let default_path = PathBuf::from(DEFAULT_CONFIG_PATH);
        if default_path.exists() {
            Some(default_path)
        } else {
            None
        }
    });
    let config = NesConfig::load_or_default(config_path.as_deref())?;
    let rom_path = rom_path_arg
        .or_else(|| config.desktop.rom_path.clone())
        .or_else(|| config.roms.smb.clone())
        .ok_or_else(|| {
            format!(
                "ROM path not configured. Provide a positional ROM argument or set `desktop.rom_path`/`roms.smb` in {DEFAULT_CONFIG_PATH}."
            )
        })?;
    Ok((rom_path, loaded_config_path))
}

fn should_quit(key: KeyEvent) -> bool {
    key_is_pressed(key.kind) && matches!(key.code, KeyCode::Esc | KeyCode::Char('q'))
}

fn key_pressed_state(kind: KeyEventKind) -> Option<bool> {
    match kind {
        KeyEventKind::Press | KeyEventKind::Repeat => Some(true),
        KeyEventKind::Release => Some(false),
    }
}

fn key_is_pressed(kind: KeyEventKind) -> bool {
    matches!(kind, KeyEventKind::Press | KeyEventKind::Repeat)
}
