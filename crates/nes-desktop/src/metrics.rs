use std::time::{Duration, Instant};

use comfy_table::{Cell, Color as TableColor, Table, presets::UTF8_FULL};
use crossterm::style::{Color, Stylize};
use nes_config::normalize_nonzero_u64;
use nes_core::NesCore;

use crate::netplay::NetplayRuntimeStats;

const DEFAULT_METRICS_EVERY_FRAMES: u64 = 60;

pub(crate) struct PerfMetrics {
    enabled: bool,
    every_n_frames: u64,
    report_start: Instant,
    report_start_ppu_frame: u64,
    report_frames: u64,
    step_work: Duration,
    render_work: Duration,
    late_frames: u64,
    last_pc: Option<u16>,
    pc_stall_frames: u64,
    last_frame_signature: Option<u64>,
    unchanged_frame_count: u64,
    warned_stall: bool,
    audio_queue_peak: usize,
    audio_queue_drops: u64,
    netplay_rtt_ms: f64,
    netplay_jitter_ms: f64,
    netplay_rollbacks: u64,
    netplay_max_rollback_distance: u64,
    netplay_desyncs: u64,
    netplay_input_delay_frames: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct MetricsSnapshot {
    pub(crate) wall_fps: f64,
    pub(crate) emu_fps: f64,
    pub(crate) avg_step_ms: f64,
    pub(crate) avg_render_ms: f64,
}

pub(crate) fn compute_metrics_snapshot(
    report_frames: u64,
    elapsed_secs: f64,
    report_start_ppu_frame: u64,
    ppu_now: u64,
    step_work: Duration,
    render_work: Duration,
) -> Option<MetricsSnapshot> {
    if report_frames == 0 || elapsed_secs <= f64::EPSILON {
        return None;
    }
    let wall_fps = report_frames as f64 / elapsed_secs;
    let emu_fps = ppu_now.saturating_sub(report_start_ppu_frame) as f64 / elapsed_secs;
    let avg_step_ms = step_work.as_secs_f64() * 1_000.0 / report_frames as f64;
    let avg_render_ms = render_work.as_secs_f64() * 1_000.0 / report_frames as f64;
    Some(MetricsSnapshot {
        wall_fps,
        emu_fps,
        avg_step_ms,
        avg_render_ms,
    })
}

/// Computes a lightweight rolling hash of the framebuffer to detect changes.
///
/// **Performance optimization:** Uses `chunks_exact(64)` rather than `.step_by(64)`
/// and indexing `rgba[idx]`. This allows the Rust compiler to elide the bounds check
/// because it knows the slice is length 64, avoiding 3,840 bounds checks per frame.
pub(crate) fn frame_signature(rgba: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for chunk in rgba.chunks_exact(64) {
        hash ^= u64::from(chunk[0]);
        hash = hash.wrapping_mul(0x0000_0001_0000_01b3);
    }
    hash
}

impl PerfMetrics {
    pub(crate) fn new(enabled: bool, every_n_frames: u64, initial_ppu_frame: u64) -> Self {
        Self {
            enabled,
            every_n_frames: normalize_nonzero_u64(every_n_frames, DEFAULT_METRICS_EVERY_FRAMES),
            report_start: Instant::now(),
            report_start_ppu_frame: initial_ppu_frame,
            report_frames: 0,
            step_work: Duration::ZERO,
            render_work: Duration::ZERO,
            late_frames: 0,
            last_pc: None,
            pc_stall_frames: 0,
            last_frame_signature: None,
            unchanged_frame_count: 0,
            warned_stall: false,
            audio_queue_peak: 0,
            audio_queue_drops: 0,
            netplay_rtt_ms: 0.0,
            netplay_jitter_ms: 0.0,
            netplay_rollbacks: 0,
            netplay_max_rollback_distance: 0,
            netplay_desyncs: 0,
            netplay_input_delay_frames: 0,
        }
    }

    pub(crate) fn on_step(
        &mut self,
        core: &NesCore,
        step_elapsed: Duration,
        missed_deadline: bool,
    ) {
        if !self.enabled {
            return;
        }
        self.report_frames = self.report_frames.saturating_add(1);
        self.step_work = self.step_work.saturating_add(step_elapsed);
        if missed_deadline {
            self.late_frames = self.late_frames.saturating_add(1);
        }
        let pc = core.cpu_pc();
        if self.last_pc == Some(pc) {
            self.pc_stall_frames = self.pc_stall_frames.saturating_add(1);
        } else {
            self.pc_stall_frames = 0;
            self.warned_stall = false;
        }
        self.last_pc = Some(pc);
        if self.pc_stall_frames >= 240 && !self.warned_stall {
            let status = core.read_memory(0x2002);
            let sprite0_y = core.ppu_oam_byte(0);
            let sprite0_x = core.ppu_oam_byte(3);
            eprintln!(
                "[warn] long pc stall detected: pc=${:04X} stall_frames={} ppu_frame={} scanline={} dot={} status={:02X} sprite0=({}, {})",
                pc,
                self.pc_stall_frames,
                core.ppu_frame_counter(),
                core.ppu_scanline(),
                core.ppu_dot(),
                status,
                sprite0_x,
                sprite0_y
            );
            self.warned_stall = true;
        }
    }

    pub(crate) fn on_render(&mut self, frame: &[u8], render_elapsed: Duration) {
        if !self.enabled {
            return;
        }
        self.render_work = self.render_work.saturating_add(render_elapsed);
        let signature = frame_signature(frame);
        if self.last_frame_signature == Some(signature) {
            self.unchanged_frame_count = self.unchanged_frame_count.saturating_add(1);
        } else {
            self.unchanged_frame_count = 0;
        }
        self.last_frame_signature = Some(signature);
    }

    pub(crate) fn maybe_report(&mut self, core: &NesCore) {
        if !self.enabled || self.report_frames < self.every_n_frames {
            return;
        }
        let elapsed = self.report_start.elapsed().as_secs_f64();
        let ppu_now = core.ppu_frame_counter();
        let Some(snapshot) = compute_metrics_snapshot(
            self.report_frames,
            elapsed,
            self.report_start_ppu_frame,
            ppu_now,
            self.step_work,
            self.render_work,
        ) else {
            return;
        };

        print_metrics_table(&snapshot, self);

        self.report_start = Instant::now();
        self.report_start_ppu_frame = ppu_now;
        self.report_frames = 0;
        self.step_work = Duration::ZERO;
        self.render_work = Duration::ZERO;
        self.late_frames = 0;
        self.audio_queue_peak = 0;
        self.audio_queue_drops = 0;
        self.netplay_rollbacks = 0;
        self.netplay_max_rollback_distance = 0;
        self.netplay_desyncs = 0;
    }

    pub(crate) fn on_audio_queue(&mut self, queue_depth: usize, dropped: bool) {
        if !self.enabled {
            return;
        }
        self.audio_queue_peak = self.audio_queue_peak.max(queue_depth);
        if dropped {
            self.audio_queue_drops = self.audio_queue_drops.saturating_add(1);
        }
    }

    pub(crate) fn on_netplay_stats(&mut self, stats: &NetplayRuntimeStats) {
        if !self.enabled {
            return;
        }
        self.netplay_rtt_ms = stats.latest_rtt_ms_or_zero();
        self.netplay_jitter_ms = stats.jitter_ms;
        self.netplay_rollbacks = stats.rollback_count;
        self.netplay_max_rollback_distance = stats.max_rollback_distance;
        self.netplay_desyncs = stats.desync_count;
        self.netplay_input_delay_frames = stats.input_delay_frames;
    }
}

fn print_metrics_table(snapshot: &MetricsSnapshot, metrics: &PerfMetrics) {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec![
        Cell::new("Metric").fg(TableColor::Cyan),
        Cell::new("Value").fg(TableColor::White),
    ]);

    table.add_row(vec![
        Cell::new("wall_fps"),
        Cell::new(format!("{:.1}", snapshot.wall_fps)).fg(TableColor::Green),
    ]);
    table.add_row(vec![
        Cell::new("emu_fps"),
        Cell::new(format!("{:.1}", snapshot.emu_fps)).fg(TableColor::Green),
    ]);
    table.add_row(vec![
        Cell::new("avg_step_ms"),
        Cell::new(format!("{:.2}", snapshot.avg_step_ms)).fg(TableColor::Yellow),
    ]);
    table.add_row(vec![
        Cell::new("avg_render_ms"),
        Cell::new(format!("{:.2}", snapshot.avg_render_ms)).fg(TableColor::Yellow),
    ]);
    table.add_row(vec![
        Cell::new("late_frames"),
        Cell::new(metrics.late_frames.to_string()).fg(if metrics.late_frames > 0 {
            TableColor::Red
        } else {
            TableColor::White
        }),
    ]);
    table.add_row(vec![
        Cell::new("pc_stall_frames"),
        Cell::new(metrics.pc_stall_frames.to_string()).fg(if metrics.pc_stall_frames > 0 {
            TableColor::Red
        } else {
            TableColor::White
        }),
    ]);
    table.add_row(vec![
        Cell::new("unchanged_frames"),
        Cell::new(metrics.unchanged_frame_count.to_string()).fg(TableColor::DarkGrey),
    ]);
    table.add_row(vec![
        Cell::new("audio_peak_q"),
        Cell::new(metrics.audio_queue_peak.to_string()),
    ]);
    table.add_row(vec![
        Cell::new("audio_drop_chunks"),
        Cell::new(metrics.audio_queue_drops.to_string()).fg(if metrics.audio_queue_drops > 0 {
            TableColor::Red
        } else {
            TableColor::White
        }),
    ]);
    table.add_row(vec![
        Cell::new("net_rtt_ms"),
        Cell::new(format!("{:.1}", metrics.netplay_rtt_ms)).fg(if metrics.netplay_rtt_ms > 100.0 {
            TableColor::Red
        } else if metrics.netplay_rtt_ms > 50.0 {
            TableColor::Yellow
        } else {
            TableColor::White
        }),
    ]);
    table.add_row(vec![
        Cell::new("net_jitter_ms"),
        Cell::new(format!("{:.1}", metrics.netplay_jitter_ms)).fg(if metrics.netplay_jitter_ms > 20.0 {
            TableColor::Red
        } else if metrics.netplay_jitter_ms > 5.0 {
            TableColor::Yellow
        } else {
            TableColor::White
        }),
    ]);
    table.add_row(vec![
        Cell::new("net_rollbacks"),
        Cell::new(metrics.netplay_rollbacks.to_string()),
    ]);
    table.add_row(vec![
        Cell::new("net_max_rb"),
        Cell::new(metrics.netplay_max_rollback_distance.to_string()),
    ]);
    table.add_row(vec![
        Cell::new("net_desyncs"),
        Cell::new(metrics.netplay_desyncs.to_string()).fg(if metrics.netplay_desyncs > 0 {
            TableColor::Red
        } else {
            TableColor::White
        }),
    ]);
    table.add_row(vec![
        Cell::new("net_delay_frames"),
        Cell::new(metrics.netplay_input_delay_frames.to_string()).fg(if metrics.netplay_input_delay_frames > 2 {
            TableColor::Red
        } else if metrics.netplay_input_delay_frames > 0 {
            TableColor::Yellow
        } else {
            TableColor::White
        }),
    ]);

    // Clear terminal and move to top left so metrics act like a dashboard
    print!("\x1B[2J\x1B[1;1H");
    println!(
        "{}\n{table}",
        " nes-desktop | Performance Dashboard "
            .with(Color::Cyan)
            .bold()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_metrics_snapshot_derives_expected_rates() {
        let snapshot = compute_metrics_snapshot(
            2,
            0.25,
            100,
            130,
            Duration::from_millis(10),
            Duration::from_millis(8),
        )
        .expect("non-zero frames and elapsed should yield snapshot");
        assert_eq!(
            snapshot,
            MetricsSnapshot {
                wall_fps: 8.0,
                emu_fps: 120.0,
                avg_step_ms: 5.0,
                avg_render_ms: 4.0,
            }
        );
    }

    #[test]
    fn compute_metrics_snapshot_handles_guard_conditions_and_saturating_ppu_delta() {
        assert!(compute_metrics_snapshot(0, 1.0, 10, 20, Duration::ZERO, Duration::ZERO).is_none());
        assert!(
            compute_metrics_snapshot(1, f64::EPSILON, 10, 20, Duration::ZERO, Duration::ZERO)
                .is_none()
        );
        let snapshot = compute_metrics_snapshot(
            1,
            1.0,
            200,
            100,
            Duration::from_millis(3),
            Duration::from_millis(2),
        )
        .expect("valid input should produce snapshot");
        assert_eq!(snapshot.emu_fps, 0.0);
    }

    #[test]
    fn perf_metrics_on_step_tracks_stalls_and_recovers_on_pc_change() {
        let core = NesCore::new();
        let mut metrics = PerfMetrics::new(true, 0, 0);
        assert_eq!(metrics.every_n_frames, super::DEFAULT_METRICS_EVERY_FRAMES);

        metrics.on_step(&core, Duration::from_millis(4), true);
        assert_eq!(metrics.report_frames, 1);
        assert_eq!(metrics.step_work, Duration::from_millis(4));
        assert_eq!(metrics.late_frames, 1);
        assert_eq!(metrics.last_pc, Some(core.cpu_pc()));
        assert_eq!(metrics.pc_stall_frames, 0);

        metrics.on_step(&core, Duration::from_millis(2), false);
        assert_eq!(metrics.report_frames, 2);
        assert_eq!(metrics.step_work, Duration::from_millis(6));
        assert_eq!(metrics.pc_stall_frames, 1);

        metrics.pc_stall_frames = 239;
        metrics.last_pc = Some(core.cpu_pc());
        metrics.warned_stall = false;
        metrics.on_step(&core, Duration::from_millis(1), false);
        assert_eq!(metrics.pc_stall_frames, 240);
        assert!(metrics.warned_stall);

        metrics.last_pc = Some(core.cpu_pc().wrapping_add(1));
        metrics.pc_stall_frames = 77;
        metrics.warned_stall = true;
        metrics.on_step(&core, Duration::from_millis(1), false);
        assert_eq!(metrics.pc_stall_frames, 0);
        assert!(!metrics.warned_stall);
    }

    #[test]
    fn perf_metrics_render_audio_and_netplay_observation_update_fields() {
        let core = NesCore::new();
        let mut metrics = PerfMetrics::new(true, 2, core.ppu_frame_counter());
        let frame = vec![0_u8; 256 * 240 * 4];

        metrics.on_render(&frame, Duration::from_millis(3));
        assert_eq!(metrics.render_work, Duration::from_millis(3));
        assert_eq!(metrics.unchanged_frame_count, 0);

        metrics.on_render(&frame, Duration::from_millis(2));
        assert_eq!(metrics.render_work, Duration::from_millis(5));
        assert_eq!(metrics.unchanged_frame_count, 1);

        metrics.on_audio_queue(3, false);
        metrics.on_audio_queue(2, true);
        assert_eq!(metrics.audio_queue_peak, 3);
        assert_eq!(metrics.audio_queue_drops, 1);

        let mut net = NetplayRuntimeStats::new(4);
        net.observe_rtt_ms(42.0);
        net.observe_rtt_ms(46.0);
        net.observe_rollback(3);
        net.observe_desync();
        metrics.on_netplay_stats(&net);
        assert_eq!(metrics.netplay_rtt_ms, 46.0);
        assert_eq!(metrics.netplay_jitter_ms, 4.0);
        assert_eq!(metrics.netplay_rollbacks, 1);
        assert_eq!(metrics.netplay_max_rollback_distance, 3);
        assert_eq!(metrics.netplay_desyncs, 1);
        assert_eq!(metrics.netplay_input_delay_frames, 4);
    }

    #[test]
    fn perf_metrics_maybe_report_resets_window_after_threshold() {
        let core = NesCore::new();
        let mut metrics = PerfMetrics::new(true, 2, core.ppu_frame_counter());
        metrics.report_frames = 2;
        metrics.step_work = Duration::from_millis(8);
        metrics.render_work = Duration::from_millis(6);
        metrics.late_frames = 1;
        metrics.audio_queue_peak = 4;
        metrics.audio_queue_drops = 2;
        metrics.netplay_rollbacks = 5;
        metrics.netplay_max_rollback_distance = 7;
        metrics.netplay_desyncs = 3;
        metrics.report_start = Instant::now() - Duration::from_millis(20);

        metrics.maybe_report(&core);

        assert_eq!(metrics.report_frames, 0);
        assert_eq!(metrics.step_work, Duration::ZERO);
        assert_eq!(metrics.render_work, Duration::ZERO);
        assert_eq!(metrics.late_frames, 0);
        assert_eq!(metrics.audio_queue_peak, 0);
        assert_eq!(metrics.audio_queue_drops, 0);
        assert_eq!(metrics.netplay_rollbacks, 0);
        assert_eq!(metrics.netplay_max_rollback_distance, 0);
        assert_eq!(metrics.netplay_desyncs, 0);
    }

    #[test]
    fn perf_metrics_maybe_report_guard_paths_skip_when_disabled_or_under_threshold() {
        let core = NesCore::new();

        let mut disabled = PerfMetrics::new(false, 1, core.ppu_frame_counter());
        disabled.report_frames = 1;
        disabled.step_work = Duration::from_millis(4);
        disabled.report_start = Instant::now() - Duration::from_millis(20);
        disabled.maybe_report(&core);
        assert_eq!(disabled.report_frames, 1);
        assert_eq!(disabled.step_work, Duration::from_millis(4));

        let mut under_threshold = PerfMetrics::new(true, 5, core.ppu_frame_counter());
        under_threshold.report_frames = 2;
        under_threshold.step_work = Duration::from_millis(6);
        under_threshold.report_start = Instant::now() - Duration::from_millis(20);
        under_threshold.maybe_report(&core);
        assert_eq!(under_threshold.report_frames, 2);
        assert_eq!(under_threshold.step_work, Duration::from_millis(6));
    }

    #[test]
    fn perf_metrics_disabled_mode_does_not_mutate_tracking_fields() {
        let core = NesCore::new();
        let frame = vec![0_u8; 256 * 240 * 4];
        let mut metrics = PerfMetrics::new(false, 1, 0);

        metrics.on_step(&core, Duration::from_millis(3), true);
        metrics.on_render(&frame, Duration::from_millis(2));
        metrics.on_audio_queue(10, true);
        let mut net = NetplayRuntimeStats::new(3);
        net.observe_rtt_ms(10.0);
        net.observe_rollback(2);
        net.observe_desync();
        metrics.on_netplay_stats(&net);
        metrics.maybe_report(&core);

        assert_eq!(metrics.report_frames, 0);
        assert_eq!(metrics.step_work, Duration::ZERO);
        assert_eq!(metrics.render_work, Duration::ZERO);
        assert_eq!(metrics.audio_queue_peak, 0);
        assert_eq!(metrics.audio_queue_drops, 0);
        assert_eq!(metrics.netplay_rtt_ms, 0.0);
        assert_eq!(metrics.netplay_rollbacks, 0);
        assert_eq!(metrics.netplay_desyncs, 0);
    }

    #[test]
    fn frame_signature_matches_reference_and_changes_on_sampled_byte() {
        let mut frame = vec![0_u8; 256];
        let signature_a = frame_signature(&frame);

        let mut reference = 0xcbf2_9ce4_8422_2325_u64;
        for idx in (0..frame.len()).step_by(64) {
            reference ^= u64::from(frame[idx]);
            reference = reference.wrapping_mul(0x0000_0001_0000_01b3);
        }
        assert_eq!(signature_a, reference);

        frame[64] = 7;
        let signature_b = frame_signature(&frame);
        assert_ne!(signature_a, signature_b);
    }
}
