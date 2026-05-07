use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use crossterm::execute;
use crossterm::style::Stylize;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use image::{DynamicImage, Rgba, RgbaImage};
use nes_config::{DEFAULT_CONFIG_PATH, NesConfig, parse_config_path_arg};
use nes_core::{Command, FRAME_HEIGHT, FRAME_RGBA_BYTES, FRAME_WIDTH, NesCore};
use nes_tui::app::map_key_event_to_command;
use nes_tui::render::{frame_lines_half_blocks, mini_palette_spans};
use ratatui::Frame;
use ratatui::Terminal;
use ratatui::backend::Backend;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui_image::errors::Errors as ImageEncodingError;
use ratatui_image::picker::{Picker, ProtocolType};
use ratatui_image::protocol::{ImageSource, StatefulProtocol, StatefulProtocolType};
use ratatui_image::{Resize, ResizeEncodeRender};

const TARGET_FRAME_TIME: Duration = Duration::from_micros(16_667);
const NES_CELL_HEIGHT: u32 = (FRAME_HEIGHT / 2) as u32;
const PROTOCOL_TARGET_FPS: u32 = 30;
const PROTOCOL_FRAME_INTERVAL: Duration = Duration::from_millis(1000 / PROTOCOL_TARGET_FPS as u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct TuiCliOptions {
    show_hud: bool,
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
        renderer: Box<ProtocolRenderer>,
        last_frame_update: Option<Instant>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LoopAction {
    Continue,
    Exit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FrameTick {
    WaitUntil(Instant),
    Step { next_deadline: Instant },
}

trait EventSource {
    fn poll_event(&mut self, timeout: Duration) -> Result<bool, String>;
    fn read_event(&mut self) -> Result<Event, String>;
}

trait LoopTimer {
    fn now(&mut self) -> Instant;
    fn sleep(&mut self, duration: Duration);
}

type PollEventFn = dyn FnMut(Duration) -> io::Result<bool>;
type ReadEventFn = dyn FnMut() -> io::Result<Event>;

struct CrosstermEventSource {
    poll_fn: Box<PollEventFn>,
    read_fn: Box<ReadEventFn>,
}

impl CrosstermEventSource {
    fn new() -> Self {
        Self {
            poll_fn: Box::new(event::poll),
            read_fn: Box::new(event::read),
        }
    }

    #[cfg(test)]
    fn with_handlers(
        poll_fn: impl FnMut(Duration) -> io::Result<bool> + 'static,
        read_fn: impl FnMut() -> io::Result<Event> + 'static,
    ) -> Self {
        Self {
            poll_fn: Box::new(poll_fn),
            read_fn: Box::new(read_fn),
        }
    }
}

impl EventSource for CrosstermEventSource {
    fn poll_event(&mut self, timeout: Duration) -> Result<bool, String> {
        (self.poll_fn)(timeout).map_err(|err| format!("Input poll failed: {err}"))
    }

    fn read_event(&mut self) -> Result<Event, String> {
        (self.read_fn)().map_err(|err| format!("Input read failed: {err}"))
    }
}

type NowFn = dyn FnMut() -> Instant;
type SleepFn = dyn FnMut(Duration);

struct SystemLoopTimer {
    now_fn: Box<NowFn>,
    sleep_fn: Box<SleepFn>,
}

impl SystemLoopTimer {
    fn new() -> Self {
        Self {
            now_fn: Box::new(Instant::now),
            sleep_fn: Box::new(std::thread::sleep),
        }
    }

    #[cfg(test)]
    fn with_handlers(
        now_fn: impl FnMut() -> Instant + 'static,
        sleep_fn: impl FnMut(Duration) + 'static,
    ) -> Self {
        Self {
            now_fn: Box::new(now_fn),
            sleep_fn: Box::new(sleep_fn),
        }
    }
}

impl LoopTimer for SystemLoopTimer {
    fn now(&mut self) -> Instant {
        (self.now_fn)()
    }

    fn sleep(&mut self, duration: Duration) {
        (self.sleep_fn)(duration);
    }
}

struct ProtocolRenderer {
    current_state: Option<StatefulProtocol>,
    request_tx: Sender<ProtocolEncodeRequest>,
    results_rx: Receiver<Result<StatefulProtocol, ImageEncodingError>>,
    pending_resize: bool,
}

struct ProtocolEncodeRequest {
    state: StatefulProtocol,
    area: Rect,
}

impl ProtocolRenderer {
    fn new(initial_state: Option<StatefulProtocol>) -> Self {
        let (tx_worker, rx_worker) = mpsc::channel::<ProtocolEncodeRequest>();
        let (tx_results, rx_results) =
            mpsc::channel::<Result<StatefulProtocol, ImageEncodingError>>();
        std::thread::spawn(move || {
            while let Ok(request) = rx_worker.recv() {
                let mut state = request.state;
                state.resize_encode(&protocol_image_resize(), request.area);
                let encoded = match state.last_encoding_result() {
                    Some(Err(err)) => Err(err),
                    _ => Ok(state),
                };
                if tx_results.send(encoded).is_err() {
                    break;
                }
            }
        });
        Self {
            current_state: initial_state,
            request_tx: tx_worker,
            results_rx: rx_results,
            pending_resize: false,
        }
    }
}

impl VideoBackend {
    fn label(&self) -> String {
        match self {
            Self::Halfblocks => "filtered half-block renderer".to_owned(),
            Self::ProtocolImage { protocol_type, .. } => {
                format!("ratatui-image ({protocol_type:?}, threaded, {PROTOCOL_TARGET_FPS}fps cap)")
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
        eprintln!("\n{}", err);
    }
}

fn run() -> Result<(), String> {
    let (rom_path, loaded_config_path, cli_options) = resolve_rom_path()?;
    let rom_bytes = fs::read(&rom_path).map_err(|err| format_rom_read_error(&rom_path, &err))?;

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

    let mut event_source = CrosstermEventSource::new();
    let mut loop_timer = SystemLoopTimer::new();
    let loop_result = event_loop(
        &mut terminal,
        &mut runtime,
        &mut event_source,
        &mut loop_timer,
    );

    disable_raw_mode().ok();
    execute!(terminal.backend_mut(), LeaveAlternateScreen).ok();
    terminal.show_cursor().ok();

    loop_result
}

fn event_loop(
    terminal: &mut Terminal<impl Backend>,
    runtime: &mut TuiRuntime,
    event_source: &mut impl EventSource,
    loop_timer: &mut impl LoopTimer,
) -> Result<(), String> {
    let mut next_frame_deadline = loop_timer.now();

    loop {
        while event_source.poll_event(Duration::from_millis(0))? {
            let ev = event_source.read_event()?;
            if let Event::Key(key) = ev
                && handle_runtime_key_event(runtime, key)? == LoopAction::Exit
            {
                return Ok(());
            }
        }

        let now = loop_timer.now();
        match evaluate_frame_tick(now, next_frame_deadline) {
            FrameTick::WaitUntil(_) => {
                loop_timer.sleep(Duration::from_millis(1));
                continue;
            }
            FrameTick::Step { next_deadline } => {
                next_frame_deadline = next_deadline;
            }
        }

        maybe_step_runtime_frame(runtime)?;
        refresh_runtime_fps(runtime, now);

        runtime.core.fill_framebuffer_rgba(&mut runtime.frame_rgba);
        draw_frame(terminal, runtime)?;
    }
}

fn handle_runtime_key_event(runtime: &mut TuiRuntime, key: KeyEvent) -> Result<LoopAction, String> {
    if should_quit(key) {
        return Ok(LoopAction::Exit);
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

    Ok(LoopAction::Continue)
}

fn evaluate_frame_tick(now: Instant, next_frame_deadline: Instant) -> FrameTick {
    if now < next_frame_deadline {
        FrameTick::WaitUntil(next_frame_deadline)
    } else {
        FrameTick::Step {
            next_deadline: now + TARGET_FRAME_TIME,
        }
    }
}

fn maybe_step_runtime_frame(runtime: &mut TuiRuntime) -> Result<(), String> {
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
    Ok(())
}

fn refresh_runtime_fps(runtime: &mut TuiRuntime, now: Instant) {
    let sample_elapsed = now.duration_since(runtime.last_fps_sample_at);
    if sample_elapsed >= Duration::from_millis(250) {
        runtime.instant_fps = runtime.frames_since_fps_sample as f64 / sample_elapsed.as_secs_f64();
        runtime.frames_since_fps_sample = 0;
        runtime.last_fps_sample_at = now;
    }
}

fn draw_frame(
    terminal: &mut Terminal<impl Backend>,
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

                if runtime.paused {
                    render_pause_overlay(f, video_viewport.area);
                }

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

            if runtime.paused {
                render_pause_overlay(f, video_viewport.area);
            }

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
            renderer: Box::new(ProtocolRenderer::new(None)),
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

fn should_replace_protocol_state(
    has_protocol_state: bool,
    pending_resize: bool,
    area_needs_resize: bool,
    paused: bool,
    last_frame_update: Option<Instant>,
    now: Instant,
    interval: Duration,
) -> bool {
    if pending_resize {
        return false;
    }
    if !has_protocol_state {
        return true;
    }
    if area_needs_resize {
        return true;
    }
    !paused && should_refresh_protocol_frame(last_frame_update, now, interval)
}

fn make_protocol_state(
    picker: &Picker,
    frame_rgba: &[u8],
    prior_protocol_type: Option<&StatefulProtocolType>,
    prior_background_color: Option<Rgba<u8>>,
) -> Option<StatefulProtocol> {
    let rgba_image = frame_rgba_to_rgba_image(frame_rgba)?;
    let image = DynamicImage::ImageRgba8(rgba_image);

    match (prior_protocol_type, prior_background_color) {
        (Some(protocol_type), Some(background_color)) => {
            let source = ImageSource::new(image, picker.font_size(), background_color);
            Some(StatefulProtocol::new(
                source,
                picker.font_size(),
                protocol_type.clone(),
            ))
        }
        _ => Some(picker.new_resize_protocol(image)),
    }
}

fn drain_protocol_results(renderer: &mut ProtocolRenderer) -> bool {
    let mut fallback_to_halfblocks = false;
    loop {
        match renderer.results_rx.try_recv() {
            Ok(result) => {
                renderer.pending_resize = false;
                match result {
                    Ok(state) => renderer.current_state = Some(state),
                    Err(_) => fallback_to_halfblocks = true,
                }
            }
            Err(TryRecvError::Empty) => return fallback_to_halfblocks,
            Err(TryRecvError::Disconnected) => return true,
        }
    }
}

fn render_video_region(frame: &mut Frame<'_>, runtime: &mut TuiRuntime, area: Rect) {
    let mut fallback_to_halfblocks = false;

    match &mut runtime.video_backend {
        VideoBackend::Halfblocks => render_halfblock_region(frame, &runtime.frame_rgba, area),
        VideoBackend::ProtocolImage {
            picker,
            renderer,
            last_frame_update,
            ..
        } => {
            fallback_to_halfblocks = drain_protocol_results(renderer);
            if fallback_to_halfblocks {
                render_halfblock_region(frame, &runtime.frame_rgba, area);
                return;
            }

            let now = Instant::now();
            let has_protocol_state = renderer.current_state.is_some();
            let area_needs_resize = renderer
                .current_state
                .as_ref()
                .and_then(|state| state.needs_resize(&protocol_image_resize(), area));
            let should_refresh = should_replace_protocol_state(
                has_protocol_state,
                renderer.pending_resize,
                area_needs_resize.is_some(),
                runtime.paused,
                *last_frame_update,
                now,
                PROTOCOL_FRAME_INTERVAL,
            );
            if should_refresh {
                let prior_protocol_type = renderer
                    .current_state
                    .as_ref()
                    .map(|state| state.protocol_type().clone());
                let prior_background_color = renderer
                    .current_state
                    .as_ref()
                    .map(StatefulProtocol::background_color);
                let Some(next_state) = make_protocol_state(
                    picker,
                    &runtime.frame_rgba,
                    prior_protocol_type.as_ref(),
                    prior_background_color,
                ) else {
                    render_halfblock_region(frame, &runtime.frame_rgba, area);
                    return;
                };
                let request = ProtocolEncodeRequest {
                    state: next_state,
                    area,
                };
                if renderer.request_tx.send(request).is_err() {
                    fallback_to_halfblocks = true;
                } else {
                    renderer.pending_resize = true;
                    *last_frame_update = Some(now);
                }
            }
            if let Some(protocol_state) = renderer.current_state.as_mut() {
                protocol_state.render(area, frame.buffer_mut());
            } else {
                render_halfblock_region(frame, &runtime.frame_rgba, area);
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
            format!("{} ROM path not configured.\n{} Provide a positional ROM argument or set `desktop.rom_path`/`roms.smb` in {DEFAULT_CONFIG_PATH}.",
                "Error:".with(crossterm::style::Color::Red).bold(),
                "Hint:".with(crossterm::style::Color::Cyan).bold())
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

fn render_pause_overlay(frame: &mut Frame<'_>, area: Rect) {
    if area.width < 14 || area.height < 3 {
        return;
    }
    let text = " [ PAUSED ] ";
    let popup_area = Rect {
        x: area.x.saturating_add((area.width.saturating_sub(14)) / 2),
        y: area.y.saturating_add((area.height.saturating_sub(3)) / 2),
        width: 14,
        height: 3,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::LightRed))
        .style(Style::default().bg(Color::Black));

    let paragraph = Paragraph::new(Line::styled(
        text,
        Style::default()
            .fg(Color::LightRed)
            .add_modifier(ratatui::style::Modifier::BOLD),
    ))
    .block(block)
    .alignment(Alignment::Center);

    frame.render_widget(Clear, popup_area);
    frame.render_widget(paragraph, popup_area);
}

fn format_rom_read_error(rom_path: &str, err: &std::io::Error) -> String {
    if err.kind() == std::io::ErrorKind::NotFound {
        format!(
            "{} Could not find the ROM file at '{}'.\n{} Check the path or try the bundled homebrew ROM: ./roms/homebrew/homebrew.nes or <path-to-your-rom>.nes",
            "Error:".with(crossterm::style::Color::Red).bold(),
            rom_path.with(crossterm::style::Color::Yellow),
            "Hint:".with(crossterm::style::Color::Cyan).bold()
        )
    } else {
        format!(
            "{} Failed to read ROM at '{}': {}",
            "Error:".with(crossterm::style::Color::Red).bold(),
            rom_path.with(crossterm::style::Color::Yellow),
            err
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CrosstermEventSource, EventSource, FrameTick, LoopAction, LoopTimer,
        PROTOCOL_FRAME_INTERVAL, ProtocolRenderer, SystemLoopTimer, TARGET_FRAME_TIME, TuiRuntime,
        VideoBackend, VideoBackendKind, drain_protocol_results, draw_frame, evaluate_frame_tick,
        event_loop, fit_nes_viewport, format_rom_read_error, frame_rgba_to_rgba_image,
        handle_runtime_key_event, key_is_pressed, key_pressed_state, make_protocol_state,
        maybe_step_runtime_frame, parse_tui_args, protocol_image_resize, refresh_runtime_fps,
        select_video_backend_kind, should_quit, should_refresh_protocol_frame,
        should_replace_protocol_state, usage_line, usage_message,
    };
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    use image::Rgba;
    use nes_core::{Button, FRAME_HEIGHT, FRAME_RGBA_BYTES, FRAME_WIDTH, NesCore};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::layout::Rect;
    use ratatui_image::{
        Resize,
        picker::{Picker, ProtocolType},
        protocol::StatefulProtocolType,
    };
    use std::collections::VecDeque;
    use std::io;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    fn key_event(code: KeyCode, kind: KeyEventKind) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind,
            state: KeyEventState::NONE,
        }
    }

    fn sample_runtime(show_hud: bool) -> TuiRuntime {
        TuiRuntime {
            core: NesCore::new(),
            rom_name: "test.nes".to_owned(),
            mapper_id: 0,
            prg_rom_bytes: 32 * 1024,
            frame_rgba: vec![0_u8; FRAME_RGBA_BYTES],
            paused: false,
            frames_rendered: 120,
            frames_since_fps_sample: 0,
            instant_fps: 59.5,
            last_fps_sample_at: Instant::now(),
            started_at: Instant::now() - Duration::from_secs(2),
            show_hud,
            video_backend: VideoBackend::Halfblocks,
        }
    }

    fn sample_ines(prg_banks: u8) -> Vec<u8> {
        let mut rom = vec![0_u8; 16 + prg_banks as usize * 16 * 1024];
        rom[0] = 0x4E; // N
        rom[1] = 0x45; // E
        rom[2] = 0x53; // S
        rom[3] = 0x1A;
        rom[4] = prg_banks;
        rom[5] = 0; // CHR RAM
        rom[6] = 0;
        rom[7] = 0;
        rom
    }

    fn sample_runtime_with_rom(show_hud: bool) -> TuiRuntime {
        let mut runtime = sample_runtime(show_hud);
        let mut rom = sample_ines(1);
        let prg_start = 16;
        rom[prg_start + 0x3FFC] = 0x00; // reset vector low
        rom[prg_start + 0x3FFD] = 0x80; // reset vector high
        runtime
            .core
            .load_ines_rom(&rom)
            .expect("sample rom should load");
        runtime
    }

    struct ScriptedEventSource {
        polls: VecDeque<bool>,
        reads: VecDeque<Event>,
        poll_calls: usize,
        read_calls: usize,
    }

    impl ScriptedEventSource {
        fn new(polls: Vec<bool>, reads: Vec<Event>) -> Self {
            Self {
                polls: polls.into(),
                reads: reads.into(),
                poll_calls: 0,
                read_calls: 0,
            }
        }
    }

    impl EventSource for ScriptedEventSource {
        fn poll_event(&mut self, _timeout: Duration) -> Result<bool, String> {
            self.poll_calls = self.poll_calls.saturating_add(1);
            self.polls
                .pop_front()
                .ok_or_else(|| "missing scripted poll result".to_owned())
        }

        fn read_event(&mut self) -> Result<Event, String> {
            self.read_calls = self.read_calls.saturating_add(1);
            self.reads
                .pop_front()
                .ok_or_else(|| "missing scripted event".to_owned())
        }
    }

    struct ScriptedLoopTimer {
        now_values: VecDeque<Instant>,
        last_now: Instant,
        sleep_calls: Vec<Duration>,
    }

    impl ScriptedLoopTimer {
        fn new(now_values: Vec<Instant>) -> Self {
            let last_now = now_values.first().copied().unwrap_or_else(Instant::now);
            Self {
                now_values: now_values.into(),
                last_now,
                sleep_calls: Vec::new(),
            }
        }
    }

    impl LoopTimer for ScriptedLoopTimer {
        fn now(&mut self) -> Instant {
            if let Some(next) = self.now_values.pop_front() {
                self.last_now = next;
                next
            } else {
                self.last_now
            }
        }

        fn sleep(&mut self, duration: Duration) {
            self.sleep_calls.push(duration);
        }
    }

    fn terminal_text(terminal: &Terminal<TestBackend>) -> String {
        let mut text = String::new();
        for cell in terminal.backend().buffer().content() {
            text.push_str(cell.symbol());
        }
        text
    }

    #[test]
    fn protocol_frame_interval_matches_target_fps() {
        assert_eq!(PROTOCOL_FRAME_INTERVAL, Duration::from_millis(33));
    }

    #[test]
    fn target_frame_time_defaults_to_60_fps() {
        assert_eq!(TARGET_FRAME_TIME, Duration::from_micros(16_667));
    }

    #[test]
    fn video_backend_label_describes_halfblock_renderer() {
        assert_eq!(
            VideoBackend::Halfblocks.label(),
            "filtered half-block renderer"
        );
    }

    #[test]
    fn usage_line_matches_cli_contract() {
        assert_eq!(
            usage_line(),
            "Usage: nes-tui [--config <path>] [--hud|--high-res] [rom_path]"
        );
    }

    #[test]
    fn usage_message_includes_usage_line_and_default_path() {
        let message = usage_message();
        assert!(message.contains(usage_line()));
        assert!(message.contains("Default config path:"));
    }

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
    fn format_rom_read_error_handles_not_found_and_other_errors() {
        let not_found = std::io::Error::from(std::io::ErrorKind::NotFound);
        let msg = format_rom_read_error("bad.nes", &not_found);
        assert!(msg.contains("Could not find the ROM file at"));
        assert!(msg.contains("bad.nes"));
        assert!(msg.contains("homebrew.nes"));

        let other = std::io::Error::from(std::io::ErrorKind::PermissionDenied);
        let msg = format_rom_read_error("bad.nes", &other);
        assert!(msg.contains("Failed to read ROM at"));
        assert!(msg.contains("bad.nes"));
        assert!(msg.contains("permission denied"));
    }

    #[test]
    fn parse_tui_args_help_flags_return_usage_message() {
        let long_help = parse_tui_args(vec!["--help".to_owned()]).expect_err("help should stop");
        assert_eq!(long_help, usage_message());

        let short_help = parse_tui_args(vec!["-h".to_owned()]).expect_err("help should stop");
        assert_eq!(short_help, usage_message());
    }

    #[test]
    fn parse_tui_args_high_res_can_disable_hud_after_enabling_it() {
        let (rom_path, options) = parse_tui_args(vec![
            "--hud".to_owned(),
            "--high-res".to_owned(),
            "rom.nes".to_owned(),
        ])
        .expect("parse should succeed");
        assert_eq!(rom_path.as_deref(), Some("rom.nes"));
        assert!(!options.show_hud);
    }

    #[test]
    fn fit_nes_viewport_returns_none_when_width_is_zero() {
        assert!(fit_nes_viewport(Rect::new(0, 0, 0, 20)).is_none());
    }

    #[test]
    fn fit_nes_viewport_returns_none_when_height_is_zero() {
        assert!(fit_nes_viewport(Rect::new(0, 0, 20, 0)).is_none());
    }

    #[test]
    fn fit_nes_viewport_uses_integer_scale_when_room_allows() {
        let area = Rect::new(0, 0, 800, 300);
        let viewport = fit_nes_viewport(area).expect("viewport should exist");
        assert_eq!(viewport.area, Rect::new(144, 30, 512, 240));
        assert_eq!(viewport.integer_scale, Some(2));
    }

    #[test]
    fn fit_nes_viewport_does_not_use_integer_scale_when_only_one_axis_fits_source() {
        let area = Rect::new(0, 0, 300, 100);
        let viewport = fit_nes_viewport(area).expect("viewport should exist");
        assert_eq!(viewport.area, Rect::new(43, 0, 213, 100));
        assert_eq!(viewport.integer_scale, None);
    }

    #[test]
    fn fit_nes_viewport_uses_width_limited_integer_scale_when_height_has_extra_room() {
        let area = Rect::new(0, 0, 600, 500);
        let viewport = fit_nes_viewport(area).expect("viewport should exist");
        assert_eq!(viewport.area, Rect::new(44, 130, 512, 240));
        assert_eq!(viewport.integer_scale, Some(2));
    }

    #[test]
    fn fit_nes_viewport_uses_height_limited_aspect_fit_when_area_is_wide_and_short() {
        let area = Rect::new(5, 7, 200, 50);
        let viewport = fit_nes_viewport(area).expect("viewport should exist");
        assert_eq!(viewport.area, Rect::new(52, 7, 106, 50));
        assert_eq!(viewport.integer_scale, None);
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
    fn should_quit_only_for_pressed_escape_or_q() {
        assert!(should_quit(key_event(KeyCode::Esc, KeyEventKind::Press)));
        assert!(should_quit(key_event(
            KeyCode::Char('q'),
            KeyEventKind::Repeat
        )));
        assert!(!should_quit(key_event(
            KeyCode::Char('q'),
            KeyEventKind::Release
        )));
        assert!(!should_quit(key_event(
            KeyCode::Char('x'),
            KeyEventKind::Press
        )));
    }

    #[test]
    fn key_pressed_state_maps_press_repeat_and_release() {
        assert_eq!(key_pressed_state(KeyEventKind::Press), Some(true));
        assert_eq!(key_pressed_state(KeyEventKind::Repeat), Some(true));
        assert_eq!(key_pressed_state(KeyEventKind::Release), Some(false));
    }

    #[test]
    fn key_is_pressed_only_for_press_or_repeat() {
        assert!(key_is_pressed(KeyEventKind::Press));
        assert!(key_is_pressed(KeyEventKind::Repeat));
        assert!(!key_is_pressed(KeyEventKind::Release));
    }

    #[test]
    fn crossterm_event_source_forwards_poll_and_read_results() {
        let mut source_true =
            CrosstermEventSource::with_handlers(|_| Ok(true), || Ok(Event::Resize(3, 2)));
        assert!(
            source_true
                .poll_event(Duration::ZERO)
                .expect("poll true should pass")
        );

        let mut source_false =
            CrosstermEventSource::with_handlers(|_| Ok(false), || Ok(Event::Resize(1, 1)));
        assert!(
            !source_false
                .poll_event(Duration::ZERO)
                .expect("poll false should pass")
        );

        let event = source_false.read_event().expect("read should pass");
        assert!(matches!(event, Event::Resize(1, 1)));
    }

    #[test]
    fn crossterm_event_source_wraps_poll_and_read_errors() {
        let mut source = CrosstermEventSource::with_handlers(
            |_| Err(io::Error::other("poll failed")),
            || Err(io::Error::other("read failed")),
        );

        let poll_err = source
            .poll_event(Duration::ZERO)
            .expect_err("poll errors should be wrapped");
        assert!(poll_err.contains("Input poll failed"));

        let read_err = source
            .read_event()
            .expect_err("read errors should be wrapped");
        assert!(read_err.contains("Input read failed"));
    }

    #[test]
    fn system_loop_timer_forwards_now_and_sleep_handlers() {
        let base = Instant::now();
        let mut tick = 0_u64;
        let sleep_calls = Arc::new(AtomicUsize::new(0));
        let sleep_calls_for_handler = Arc::clone(&sleep_calls);
        let mut timer = SystemLoopTimer::with_handlers(
            move || {
                let now = base + Duration::from_millis(tick);
                tick = tick.saturating_add(1);
                now
            },
            move |_| {
                sleep_calls_for_handler.fetch_add(1, Ordering::SeqCst);
            },
        );

        let first = timer.now();
        let second = timer.now();
        assert!(second > first);

        timer.sleep(Duration::from_millis(5));
        assert_eq!(sleep_calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn handle_runtime_key_event_recognizes_quit_inputs() {
        let mut runtime = sample_runtime(false);
        let esc =
            handle_runtime_key_event(&mut runtime, key_event(KeyCode::Esc, KeyEventKind::Press))
                .expect("escape should not error");
        assert_eq!(esc, LoopAction::Exit);

        let q_repeat = handle_runtime_key_event(
            &mut runtime,
            key_event(KeyCode::Char('q'), KeyEventKind::Repeat),
        )
        .expect("q repeat should not error");
        assert_eq!(q_repeat, LoopAction::Exit);
    }

    #[test]
    fn handle_runtime_key_event_toggles_pause_only_for_pressed_p() {
        let mut runtime = sample_runtime(false);

        handle_runtime_key_event(
            &mut runtime,
            key_event(KeyCode::Char('p'), KeyEventKind::Release),
        )
        .expect("release should not fail");
        assert!(!runtime.paused);

        handle_runtime_key_event(
            &mut runtime,
            key_event(KeyCode::Char('x'), KeyEventKind::Press),
        )
        .expect("other key should not fail");
        assert!(!runtime.paused);

        handle_runtime_key_event(
            &mut runtime,
            key_event(KeyCode::Char('p'), KeyEventKind::Press),
        )
        .expect("press should toggle pause on");
        assert!(runtime.paused);

        handle_runtime_key_event(
            &mut runtime,
            key_event(KeyCode::Char('p'), KeyEventKind::Repeat),
        )
        .expect("repeat should toggle pause off");
        assert!(!runtime.paused);
    }

    #[test]
    fn handle_runtime_key_event_resets_only_for_pressed_r() {
        let mut runtime = sample_runtime_with_rom(false);
        runtime.paused = false;
        maybe_step_runtime_frame(&mut runtime).expect("step should succeed");
        let stepped_pc = runtime.core.cpu_pc();
        assert_ne!(stepped_pc, 0x8000);

        handle_runtime_key_event(
            &mut runtime,
            key_event(KeyCode::Char('x'), KeyEventKind::Press),
        )
        .expect("unrelated key should not error");
        assert_eq!(runtime.core.cpu_pc(), stepped_pc);

        handle_runtime_key_event(
            &mut runtime,
            key_event(KeyCode::Char('r'), KeyEventKind::Press),
        )
        .expect("reset key should not error");
        assert_eq!(runtime.core.cpu_pc(), 0x8000);
    }

    #[test]
    fn handle_runtime_key_event_toggles_hud_only_for_pressed_i() {
        let mut runtime = sample_runtime(false);
        assert!(!runtime.show_hud);

        handle_runtime_key_event(
            &mut runtime,
            key_event(KeyCode::Char('i'), KeyEventKind::Release),
        )
        .expect("release should not toggle");
        assert!(!runtime.show_hud);

        handle_runtime_key_event(
            &mut runtime,
            key_event(KeyCode::Char('x'), KeyEventKind::Press),
        )
        .expect("other key should not toggle");
        assert!(!runtime.show_hud);

        handle_runtime_key_event(
            &mut runtime,
            key_event(KeyCode::Char('i'), KeyEventKind::Press),
        )
        .expect("lowercase i should toggle on");
        assert!(runtime.show_hud);

        handle_runtime_key_event(
            &mut runtime,
            key_event(KeyCode::Char('I'), KeyEventKind::Press),
        )
        .expect("uppercase I should toggle off");
        assert!(!runtime.show_hud);
    }

    #[test]
    fn handle_runtime_key_event_applies_controller_mapping_for_press_and_release() {
        let mut runtime = sample_runtime(false);
        handle_runtime_key_event(
            &mut runtime,
            key_event(KeyCode::Char('z'), KeyEventKind::Press),
        )
        .expect("controller press should succeed");
        assert_eq!(runtime.core.controller_bits(), Button::A.bit_mask());

        handle_runtime_key_event(
            &mut runtime,
            key_event(KeyCode::Char('z'), KeyEventKind::Release),
        )
        .expect("controller release should succeed");
        assert_eq!(runtime.core.controller_bits(), 0);
    }

    #[test]
    fn evaluate_frame_tick_covers_wait_and_step_paths() {
        let now = Instant::now();
        let future = now + Duration::from_millis(2);
        assert_eq!(
            evaluate_frame_tick(now, future),
            FrameTick::WaitUntil(future)
        );

        match evaluate_frame_tick(now, now) {
            FrameTick::Step { next_deadline } => assert_eq!(next_deadline, now + TARGET_FRAME_TIME),
            FrameTick::WaitUntil(_) => panic!("expected step for now==deadline"),
        }

        let late_now = now + Duration::from_millis(1);
        match evaluate_frame_tick(late_now, now) {
            FrameTick::Step { next_deadline } => {
                assert_eq!(next_deadline, late_now + TARGET_FRAME_TIME)
            }
            FrameTick::WaitUntil(_) => panic!("expected step for late frame"),
        }
    }

    #[test]
    fn maybe_step_runtime_frame_respects_paused_state_and_counts() {
        let mut paused_runtime = sample_runtime(false);
        paused_runtime.paused = true;
        let paused_frames = paused_runtime.frames_rendered;
        maybe_step_runtime_frame(&mut paused_runtime).expect("paused branch should be a no-op");
        assert_eq!(paused_runtime.frames_rendered, paused_frames);
        assert_eq!(paused_runtime.frames_since_fps_sample, 0);

        let mut running_runtime = sample_runtime_with_rom(false);
        running_runtime.paused = false;
        running_runtime.frames_rendered = 0;
        running_runtime.frames_since_fps_sample = 0;
        maybe_step_runtime_frame(&mut running_runtime).expect("running branch should step frame");
        assert_eq!(running_runtime.frames_rendered, 1);
        assert_eq!(running_runtime.frames_since_fps_sample, 1);
    }

    #[test]
    fn refresh_runtime_fps_updates_only_after_sampling_interval() {
        let now = Instant::now();
        let mut runtime = sample_runtime(false);
        runtime.frames_since_fps_sample = 120;
        runtime.last_fps_sample_at = now - Duration::from_millis(300);

        refresh_runtime_fps(&mut runtime, now);
        assert!((runtime.instant_fps - 400.0).abs() < 0.000_001);
        assert_eq!(runtime.frames_since_fps_sample, 0);
        assert_eq!(runtime.last_fps_sample_at, now);

        runtime.frames_since_fps_sample = 60;
        runtime.last_fps_sample_at = now;
        let fps_before = runtime.instant_fps;
        let not_yet = now + Duration::from_millis(100);
        refresh_runtime_fps(&mut runtime, not_yet);
        assert_eq!(runtime.instant_fps, fps_before);
        assert_eq!(runtime.frames_since_fps_sample, 60);
        assert_eq!(runtime.last_fps_sample_at, now);
    }

    #[test]
    fn event_loop_processes_frame_then_exits_on_scripted_quit_event() {
        let mut terminal = Terminal::new(TestBackend::new(80, 40)).expect("terminal should build");
        let mut runtime = sample_runtime(false);
        runtime.paused = true;

        let mut event_source = ScriptedEventSource::new(
            vec![false, true, true],
            vec![
                Event::Key(key_event(KeyCode::Char('x'), KeyEventKind::Press)),
                Event::Key(key_event(KeyCode::Char('q'), KeyEventKind::Press)),
            ],
        );
        let now = Instant::now();
        let mut loop_timer = ScriptedLoopTimer::new(vec![now, now]);

        event_loop(
            &mut terminal,
            &mut runtime,
            &mut event_source,
            &mut loop_timer,
        )
        .expect("event loop should exit cleanly");
        assert!(
            event_source.poll_calls >= 3,
            "event loop should poll once for frame and twice for scripted keys"
        );
        assert_eq!(event_source.read_calls, 2);
        assert!(
            loop_timer.sleep_calls.is_empty(),
            "now==deadline should step instead of sleeping"
        );
    }

    #[test]
    fn make_protocol_state_reuses_protocol_variant_from_previous_state() {
        let mut picker = Picker::from_fontsize((8, 16));
        picker.set_protocol_type(ProtocolType::Halfblocks);
        let frame = vec![0_u8; FRAME_RGBA_BYTES];
        let first = make_protocol_state(&picker, &frame, None, None).expect("state should build");
        let second = make_protocol_state(
            &picker,
            &frame,
            Some(first.protocol_type()),
            Some(first.background_color()),
        )
        .expect("state should build");
        assert!(matches!(
            first.protocol_type(),
            ratatui_image::protocol::StatefulProtocolType::Halfblocks(_)
        ));
        assert!(matches!(
            second.protocol_type(),
            ratatui_image::protocol::StatefulProtocolType::Halfblocks(_)
        ));
    }

    #[test]
    fn make_protocol_state_preserves_prior_background_when_reusing_protocol_type() {
        let mut picker = Picker::from_fontsize((8, 16));
        picker.set_protocol_type(ProtocolType::Halfblocks);
        let frame = vec![0_u8; FRAME_RGBA_BYTES];
        let first = make_protocol_state(&picker, &frame, None, None).expect("state should build");
        let forced_background = Rgba([1, 2, 3, 255]);
        let second = make_protocol_state(
            &picker,
            &frame,
            Some(first.protocol_type()),
            Some(forced_background),
        )
        .expect("state should build");
        assert!(matches!(
            second.protocol_type(),
            StatefulProtocolType::Halfblocks(_)
        ));
        assert_eq!(second.background_color(), forced_background);
    }

    #[test]
    fn protocol_state_replace_skips_when_pending_resize_is_active() {
        assert!(!should_replace_protocol_state(
            true,
            true,
            false,
            false,
            Some(Instant::now() - Duration::from_millis(34)),
            Instant::now(),
            Duration::from_millis(33)
        ));
    }

    #[test]
    fn protocol_state_replace_happens_without_existing_state() {
        assert!(should_replace_protocol_state(
            false,
            false,
            false,
            true,
            None,
            Instant::now(),
            Duration::from_millis(33)
        ));
    }

    #[test]
    fn protocol_state_replace_stays_false_when_paused_even_if_refresh_due() {
        let now = Instant::now();
        let interval = Duration::from_millis(33);
        assert!(!should_replace_protocol_state(
            true,
            false,
            false,
            true,
            Some(now - interval),
            now,
            interval
        ));
    }

    #[test]
    fn protocol_state_replace_stays_false_when_interval_has_not_elapsed() {
        let now = Instant::now();
        let interval = Duration::from_millis(33);
        assert!(!should_replace_protocol_state(
            true,
            false,
            false,
            false,
            Some(now - Duration::from_millis(10)),
            now,
            interval
        ));
    }

    #[test]
    fn protocol_state_replace_happens_when_area_needs_resize() {
        assert!(should_replace_protocol_state(
            true,
            false,
            true,
            true,
            Some(Instant::now()),
            Instant::now(),
            Duration::from_millis(33)
        ));
    }

    #[test]
    fn drain_protocol_results_returns_false_when_channel_is_empty() {
        let (request_tx, _request_rx) = mpsc::channel();
        let (results_tx, results_rx) = mpsc::channel();
        let mut renderer = ProtocolRenderer {
            current_state: None,
            request_tx,
            results_rx,
            pending_resize: true,
        };

        assert!(!drain_protocol_results(&mut renderer));
        assert!(renderer.pending_resize);

        drop(results_tx);
    }

    #[test]
    fn drain_protocol_results_returns_true_when_worker_channel_disconnects() {
        let (request_tx, _request_rx) = mpsc::channel();
        let (results_tx, results_rx) = mpsc::channel();
        drop(results_tx);
        let mut renderer = ProtocolRenderer {
            current_state: None,
            request_tx,
            results_rx,
            pending_resize: true,
        };

        assert!(drain_protocol_results(&mut renderer));
    }

    #[test]
    fn drain_protocol_results_applies_state_and_clears_pending_resize() {
        let mut picker = Picker::from_fontsize((8, 16));
        picker.set_protocol_type(ProtocolType::Halfblocks);
        let frame = vec![0_u8; FRAME_RGBA_BYTES];
        let state = make_protocol_state(&picker, &frame, None, None).expect("state should build");

        let (request_tx, _request_rx) = mpsc::channel();
        let (results_tx, results_rx) = mpsc::channel();
        results_tx
            .send(Ok(state))
            .expect("sending synthetic state should succeed");

        let mut renderer = ProtocolRenderer {
            current_state: None,
            request_tx,
            results_rx,
            pending_resize: true,
        };

        assert!(!drain_protocol_results(&mut renderer));
        assert!(renderer.current_state.is_some());
        assert!(!renderer.pending_resize);
    }

    #[test]
    fn draw_frame_renders_video_glyphs_when_hud_is_hidden() {
        let mut terminal = Terminal::new(TestBackend::new(80, 40)).expect("terminal should build");
        let mut runtime = sample_runtime(false);
        draw_frame(&mut terminal, &mut runtime).expect("draw should succeed");

        let has_halfblock_glyph = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .any(|cell| cell.symbol() == "\u{2580}");
        assert!(has_halfblock_glyph);
    }

    #[test]
    fn draw_frame_hud_path_contains_header_controls_and_expected_avg_band() {
        let mut terminal =
            Terminal::new(TestBackend::new(140, 50)).expect("terminal should initialize");
        let mut runtime = sample_runtime(true);
        runtime.rom_name = "zelda.nes".to_owned();
        runtime.frames_rendered = 120;
        runtime.started_at = Instant::now() - Duration::from_secs(2);

        draw_frame(&mut terminal, &mut runtime).expect("draw should succeed");
        let text = terminal_text(&terminal);
        assert!(text.contains("NES-TUI"));
        assert!(text.contains("zelda.nes"));
        assert!(text.contains("P=Pause  R=Reset  Q/Esc=Quit"));
        assert!(
            text.contains("avg 59.") || text.contains("avg 60.") || text.contains("avg 61."),
            "expected avg fps near 60.0, got: {text}"
        );
    }
}

#[cfg(test)]
mod tests_tui_render {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use ratatui::layout::Rect;

    #[test]
    fn test_render_pause_overlay_boundaries() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        // Large enough
        terminal.draw(|f| {
            render_pause_overlay(f, Rect::new(0, 0, 14, 3));
        }).unwrap();
        let buffer = terminal.backend().buffer();
        assert!(buffer.content.iter().any(|c| c.symbol() == "P")); // "PAUSED"

        // Too small width
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| {
            render_pause_overlay(f, Rect::new(0, 0, 13, 3));
        }).unwrap();
        let buffer = terminal.backend().buffer();
        assert!(!buffer.content.iter().any(|c| c.symbol() == "P"));

        // Too small height
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| {
            render_pause_overlay(f, Rect::new(0, 0, 14, 2));
        }).unwrap();
        let buffer = terminal.backend().buffer();
        assert!(!buffer.content.iter().any(|c| c.symbol() == "P"));

        // Ensure centering math is covered
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| {
            render_pause_overlay(f, Rect::new(10, 5, 20, 10)); // odd/even to check `+` vs `-` vs `/ 2`
        }).unwrap();
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer.cell((17, 9)).unwrap().symbol(), "P");
    }
}
