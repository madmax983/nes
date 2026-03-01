use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use image::{DynamicImage, RgbaImage};
use nes_config::{DEFAULT_CONFIG_PATH, NesConfig, parse_config_path_arg};
use nes_core::{Command, FRAME_HEIGHT, FRAME_RGBA_BYTES, FRAME_WIDTH, NesCore};
use nes_tui::app::map_key_event_to_command;
use nes_tui::render::{frame_lines_half_blocks, mini_palette_spans};
use ratatui::Frame;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui_image::picker::{Picker, ProtocolType};
use ratatui_image::protocol::{ImageSource, StatefulProtocol};
use ratatui_image::{Resize, StatefulImage};

const TARGET_FRAME_TIME: Duration = Duration::from_micros(16_667);
const NES_CELL_HEIGHT: u32 = (FRAME_HEIGHT / 2) as u32;
const PROTOCOL_TARGET_FPS: u32 = 30;
const PROTOCOL_FRAME_INTERVAL: Duration = Duration::from_millis(1000 / PROTOCOL_TARGET_FPS as u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TuiCliOptions {
    show_hud: bool,
}

impl Default for TuiCliOptions {
    fn default() -> Self {
        Self { show_hud: false }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct VideoViewport {
    area: Rect,
    integer_scale: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VideoBackendKind {
    Halfblocks,
    ProtocolImage,
}

enum VideoBackend {
    Halfblocks,
    ProtocolImage {
        protocol_type: ProtocolType,
        picker: Picker,
        state: Option<ratatui_image::protocol::StatefulProtocol>,
        last_frame_update: Option<Instant>,
    },
}

impl VideoBackend {
    fn label(&self) -> String {
        match self {
            Self::Halfblocks => "filtered half-block renderer".to_owned(),
            Self::ProtocolImage { protocol_type, .. } => {
                format!("ratatui-image ({protocol_type:?}, {PROTOCOL_TARGET_FPS}fps cap)")
            }
        }
    }
}

struct TuiRuntime {
    core: NesCore,
    rom_name: String,
    mapper_id: u8,
    prg_rom_bytes: usize,
    frame_rgba: Vec<u8>,
    paused: bool,
    frames_rendered: u64,
    frames_since_fps_sample: u64,
    instant_fps: f64,
    last_fps_sample_at: Instant,
    started_at: Instant,
    show_hud: bool,
    video_backend: VideoBackend,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
    }
}

fn run() -> Result<(), String> {
    let (rom_path, loaded_config_path, cli_options) = resolve_rom_path()?;
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
        frames_since_fps_sample: 0,
        instant_fps: 0.0,
        last_fps_sample_at: Instant::now(),
        started_at: Instant::now(),
        show_hud: cli_options.show_hud,
        video_backend: VideoBackend::Halfblocks,
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

    runtime.video_backend = detect_video_backend();
    eprintln!("Video backend: {}", runtime.video_backend.label());

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
                } else if key_is_pressed(key.kind)
                    && matches!(key.code, KeyCode::Char('i') | KeyCode::Char('I'))
                {
                    runtime.show_hud = !runtime.show_hud;
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
            runtime.frames_since_fps_sample = runtime.frames_since_fps_sample.saturating_add(1);
        }

        let sample_elapsed = now.duration_since(runtime.last_fps_sample_at);
        if sample_elapsed >= Duration::from_millis(250) {
            runtime.instant_fps =
                runtime.frames_since_fps_sample as f64 / sample_elapsed.as_secs_f64();
            runtime.frames_since_fps_sample = 0;
            runtime.last_fps_sample_at = now;
        }

        runtime.core.fill_framebuffer_rgba(&mut runtime.frame_rgba);
        draw_frame(terminal, runtime)?;
    }
}

fn draw_frame(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    runtime: &mut TuiRuntime,
) -> Result<(), String> {
    terminal
        .draw(|f| {
            let status = if runtime.paused { "paused" } else { "running" };
            let elapsed = runtime.started_at.elapsed().as_secs_f64().max(1e-6);
            let average_fps = runtime.frames_rendered as f64 / elapsed;
            let backend_label = runtime.video_backend.label();
            if !runtime.show_hud {
                let full_area = f.area();
                let Some(video_viewport) = fit_nes_viewport(full_area) else {
                    return;
                };

                let matte = Block::default().style(Style::default().bg(Color::Black));
                f.render_widget(matte, full_area);
                render_video_region(f, runtime, video_viewport.area);

                return;
            }

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
            let header = Paragraph::new(vec![
                Line::styled(
                    format!("NES-TUI | {backend_label}"),
                    Style::default().fg(Color::Cyan),
                ),
                Line::styled(
                    format!(
                        "{} | mapper {} | frame {} | fps {:.1} | avg {:.1} | {status}",
                        runtime.rom_name,
                        runtime.mapper_id,
                        runtime.frames_rendered,
                        runtime.instant_fps,
                        average_fps,
                    ),
                    Style::default().fg(Color::White),
                ),
            ]);
            f.render_widget(header, root[0]);

            let video_area = root[1];
            let panel_area = root[2];
            let footer_area = root[3];

            let video_block = Block::default()
                .title(Span::styled(" Screen ", Style::default().fg(Color::Green)))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray));
            let video_inner = video_block.inner(video_area);
            f.render_widget(video_block, video_area);

            let Some(video_viewport) = fit_nes_viewport(video_inner) else {
                return;
            };
            let matte = Block::default().style(Style::default().bg(Color::Black));
            f.render_widget(matte, video_inner);
            render_video_region(f, runtime, video_viewport.area);

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
                Line::styled("Arrows=DPAD  Z=A  X=B", Style::default().fg(Color::Gray)),
                Line::styled(
                    "Enter=Start  Tab/C=Select",
                    Style::default().fg(Color::Gray),
                ),
                Line::styled(
                    "P=Pause  R=Reset  Q/Esc=Quit",
                    Style::default().fg(Color::Gray),
                ),
                Line::styled("I=Toggle HUD", Style::default().fg(Color::Gray)),
            ];
            let controls_widget = Paragraph::new(controls).block(
                Block::default()
                    .title(Span::styled(" Controls ", Style::default().fg(Color::Cyan)))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)),
            );
            f.render_widget(controls_widget, panels[1]);

            let footer = Paragraph::new(Line::styled(
                "Tip: press I to hide HUD and reclaim rows for higher-res video (aspect-locked).",
                Style::default().fg(Color::DarkGray),
            ));
            f.render_widget(footer, footer_area);
        })
        .map(|_| ())
        .map_err(|err| format!("Render failed: {err}"))
}

fn detect_video_backend() -> VideoBackend {
    let Ok(picker) = Picker::from_query_stdio() else {
        return VideoBackend::Halfblocks;
    };
    let protocol_type = picker.protocol_type();
    match select_video_backend_kind(Some(protocol_type)) {
        VideoBackendKind::Halfblocks => VideoBackend::Halfblocks,
        VideoBackendKind::ProtocolImage => VideoBackend::ProtocolImage {
            protocol_type,
            picker,
            state: None,
            last_frame_update: None,
        },
    }
}

fn select_video_backend_kind(protocol_type: Option<ProtocolType>) -> VideoBackendKind {
    match protocol_type {
        Some(ProtocolType::Halfblocks) | None => VideoBackendKind::Halfblocks,
        Some(_) => VideoBackendKind::ProtocolImage,
    }
}

fn frame_rgba_to_rgba_image(frame_rgba: &[u8]) -> Option<RgbaImage> {
    if frame_rgba.len() != FRAME_RGBA_BYTES {
        return None;
    }
    RgbaImage::from_raw(FRAME_WIDTH as u32, FRAME_HEIGHT as u32, frame_rgba.to_vec())
}

fn should_refresh_protocol_frame(
    last_frame_update: Option<Instant>,
    now: Instant,
    interval: Duration,
) -> bool {
    match last_frame_update {
        None => true,
        Some(last) => now.duration_since(last) >= interval,
    }
}

fn make_protocol_state(
    picker: &Picker,
    frame_rgba: &[u8],
    prior_state: Option<&StatefulProtocol>,
) -> Option<StatefulProtocol> {
    let rgba_image = frame_rgba_to_rgba_image(frame_rgba)?;
    let image = DynamicImage::ImageRgba8(rgba_image);

    match prior_state {
        Some(previous) => {
            let source = ImageSource::new(image, picker.font_size(), previous.background_color());
            Some(StatefulProtocol::new(
                source,
                picker.font_size(),
                previous.protocol_type().clone(),
            ))
        }
        None => Some(picker.new_resize_protocol(image)),
    }
}

fn render_video_region(frame: &mut Frame<'_>, runtime: &mut TuiRuntime, area: Rect) {
    let mut fallback_to_halfblocks = false;

    match &mut runtime.video_backend {
        VideoBackend::Halfblocks => render_halfblock_region(frame, &runtime.frame_rgba, area),
        VideoBackend::ProtocolImage {
            picker,
            state,
            last_frame_update,
            ..
        } => {
            let now = Instant::now();
            let should_refresh = state.is_none()
                || (!runtime.paused
                    && should_refresh_protocol_frame(
                        *last_frame_update,
                        now,
                        PROTOCOL_FRAME_INTERVAL,
                    ));
            if should_refresh {
                let Some(next_state) =
                    make_protocol_state(picker, &runtime.frame_rgba, state.as_ref())
                else {
                    render_halfblock_region(frame, &runtime.frame_rgba, area);
                    return;
                };
                *state = Some(next_state);
                *last_frame_update = Some(now);
            }
            let Some(protocol_state) = state.as_mut() else {
                render_halfblock_region(frame, &runtime.frame_rgba, area);
                return;
            };
            frame.render_stateful_widget(protocol_image_widget(), area, protocol_state);
            if let Some(result) = protocol_state.last_encoding_result() {
                if result.is_err() {
                    fallback_to_halfblocks = true;
                }
            }
        }
    }

    if fallback_to_halfblocks {
        runtime.video_backend = VideoBackend::Halfblocks;
        render_halfblock_region(frame, &runtime.frame_rgba, area);
    }
}

fn render_halfblock_region(frame: &mut Frame<'_>, frame_rgba: &[u8], area: Rect) {
    let video_lines = frame_lines_half_blocks(frame_rgba, area.width, area.height);
    frame.render_widget(Paragraph::new(video_lines), area);
}

fn protocol_image_widget() -> StatefulImage<ratatui_image::protocol::StatefulProtocol> {
    StatefulImage::default().resize(protocol_image_resize())
}

fn protocol_image_resize() -> Resize {
    Resize::Scale(None)
}

fn fit_nes_viewport(area: Rect) -> Option<VideoViewport> {
    if area.width == 0 || area.height == 0 {
        return None;
    }

    let avail_w = u32::from(area.width);
    let avail_h = u32::from(area.height);
    let source_w = FRAME_WIDTH as u32;
    let source_h = NES_CELL_HEIGHT;

    let (target_w, target_h, integer_scale) = if avail_w >= source_w && avail_h >= source_h {
        let scale = (avail_w / source_w).min(avail_h / source_h);
        let scale = scale.max(1);
        (
            source_w.saturating_mul(scale),
            source_h.saturating_mul(scale),
            Some(scale),
        )
    } else if avail_w.saturating_mul(source_h) <= avail_h.saturating_mul(source_w) {
        (
            avail_w,
            (avail_w.saturating_mul(source_h) / source_w).max(1),
            None,
        )
    } else {
        (
            (avail_h.saturating_mul(source_w) / source_h).max(1),
            avail_h,
            None,
        )
    };

    let fitted_w = target_w.min(avail_w) as u16;
    let fitted_h = target_h.min(avail_h) as u16;
    let x = area
        .x
        .saturating_add((area.width.saturating_sub(fitted_w)) / 2);
    let y = area
        .y
        .saturating_add((area.height.saturating_sub(fitted_h)) / 2);

    Some(VideoViewport {
        area: Rect::new(x, y, fitted_w, fitted_h),
        integer_scale,
    })
}

fn usage_line() -> &'static str {
    "Usage: nes-tui [--config <path>] [--hud|--high-res] [rom_path]"
}

fn usage_message() -> String {
    format!(
        "{usage}\nDefault config path: {DEFAULT_CONFIG_PATH}",
        usage = usage_line()
    )
}

fn parse_tui_args(pass_through: Vec<String>) -> Result<(Option<String>, TuiCliOptions), String> {
    let mut options = TuiCliOptions::default();
    let mut rom_path_arg = None::<String>;

    for arg in pass_through {
        match arg.as_str() {
            "--help" | "-h" => return Err(usage_message()),
            "--hud" => {
                options.show_hud = true;
                continue;
            }
            "--high-res" => {
                options.show_hud = false;
                continue;
            }
            _ => {}
        }

        if arg.starts_with("--") {
            return Err(format!("unknown flag '{arg}'. {}", usage_line()));
        }
        if rom_path_arg.is_some() {
            return Err(
                "multiple ROM paths provided; expected at most one positional ROM path".to_owned(),
            );
        }
        rom_path_arg = Some(arg);
    }

    Ok((rom_path_arg, options))
}

fn resolve_rom_path() -> Result<(String, Option<PathBuf>, TuiCliOptions), String> {
    let raw_args: Vec<String> = std::env::args().skip(1).collect();
    let (config_path, pass_through) = parse_config_path_arg(&raw_args)?;
    let (rom_path_arg, cli_options) = parse_tui_args(pass_through)?;

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
    Ok((rom_path, loaded_config_path, cli_options))
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

#[cfg(test)]
mod tests {
    use super::{
        VideoBackendKind, fit_nes_viewport, frame_rgba_to_rgba_image, make_protocol_state,
        parse_tui_args, protocol_image_resize, select_video_backend_kind,
        should_refresh_protocol_frame,
    };
    use nes_core::{FRAME_HEIGHT, FRAME_RGBA_BYTES, FRAME_WIDTH};
    use ratatui::layout::Rect;
    use ratatui_image::{
        Resize,
        picker::{Picker, ProtocolType},
    };
    use std::time::{Duration, Instant};

    #[test]
    fn parse_tui_args_defaults_to_high_res_mode() {
        let (rom_path, options) =
            parse_tui_args(vec!["rom.nes".to_owned()]).expect("parse should succeed");
        assert_eq!(rom_path.as_deref(), Some("rom.nes"));
        assert!(!options.show_hud);
    }

    #[test]
    fn parse_tui_args_allows_hud_opt_in() {
        let (rom_path, options) = parse_tui_args(vec!["--hud".to_owned(), "rom.nes".to_owned()])
            .expect("parse should succeed");
        assert_eq!(rom_path.as_deref(), Some("rom.nes"));
        assert!(options.show_hud);
    }

    #[test]
    fn parse_tui_args_rejects_unknown_flag() {
        let err = parse_tui_args(vec!["--bogus".to_owned()]).expect_err("parse should fail");
        assert!(err.contains("unknown flag '--bogus'"));
    }

    #[test]
    fn fit_nes_viewport_uses_integer_scale_when_room_allows() {
        let area = Rect::new(0, 0, 800, 300);
        let viewport = fit_nes_viewport(area).expect("viewport should exist");
        assert_eq!(viewport.area, Rect::new(144, 30, 512, 240));
        assert_eq!(viewport.integer_scale, Some(2));
    }

    #[test]
    fn fit_nes_viewport_aspect_fits_when_too_small_for_integer_scale() {
        let area = Rect::new(10, 5, 100, 50);
        let viewport = fit_nes_viewport(area).expect("viewport should exist");
        assert_eq!(viewport.area, Rect::new(10, 7, 100, 46));
        assert_eq!(viewport.integer_scale, None);
    }

    #[test]
    fn video_backend_selection_falls_back_for_halfblocks() {
        assert_eq!(
            select_video_backend_kind(Some(ProtocolType::Halfblocks)),
            VideoBackendKind::Halfblocks
        );
        assert_eq!(
            select_video_backend_kind(None),
            VideoBackendKind::Halfblocks
        );
    }

    #[test]
    fn video_backend_selection_uses_protocol_for_non_halfblock() {
        assert_eq!(
            select_video_backend_kind(Some(ProtocolType::Sixel)),
            VideoBackendKind::ProtocolImage
        );
    }

    #[test]
    fn frame_rgba_to_image_preserves_dimensions_and_first_pixel() {
        let mut frame = vec![0_u8; FRAME_RGBA_BYTES];
        frame[0] = 10;
        frame[1] = 20;
        frame[2] = 30;
        frame[3] = 255;
        let image = frame_rgba_to_rgba_image(&frame).expect("frame should convert");
        assert_eq!(image.width(), FRAME_WIDTH as u32);
        assert_eq!(image.height(), FRAME_HEIGHT as u32);
        let p = image.get_pixel(0, 0);
        assert_eq!([p[0], p[1], p[2], p[3]], [10, 20, 30, 255]);
    }

    #[test]
    fn protocol_image_widget_uses_scale_mode() {
        assert!(matches!(protocol_image_resize(), Resize::Scale(None)));
    }

    #[test]
    fn protocol_frame_refreshes_when_no_prior_timestamp() {
        assert!(should_refresh_protocol_frame(
            None,
            Instant::now(),
            Duration::from_millis(33)
        ));
    }

    #[test]
    fn protocol_frame_does_not_refresh_before_interval_elapsed() {
        let now = Instant::now();
        let interval = Duration::from_millis(33);
        let recent = now - Duration::from_millis(20);
        assert!(!should_refresh_protocol_frame(Some(recent), now, interval));
    }

    #[test]
    fn protocol_frame_refreshes_once_interval_elapsed() {
        let now = Instant::now();
        let interval = Duration::from_millis(33);
        let old = now - interval;
        assert!(should_refresh_protocol_frame(Some(old), now, interval));
    }

    #[test]
    fn make_protocol_state_reuses_protocol_variant_from_previous_state() {
        let mut picker = Picker::from_fontsize((8, 16));
        picker.set_protocol_type(ProtocolType::Halfblocks);
        let frame = vec![0_u8; FRAME_RGBA_BYTES];
        let first = make_protocol_state(&picker, &frame, None).expect("state should build");
        let second =
            make_protocol_state(&picker, &frame, Some(&first)).expect("state should build");
        assert!(matches!(
            first.protocol_type(),
            ratatui_image::protocol::StatefulProtocolType::Halfblocks(_)
        ));
        assert!(matches!(
            second.protocol_type(),
            ratatui_image::protocol::StatefulProtocolType::Halfblocks(_)
        ));
    }
}
