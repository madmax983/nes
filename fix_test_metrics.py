import re

with open("crates/nes-desktop/src/metrics.rs", "r") as f:
    content = f.read()

test_func = """
    #[test]
    fn disabled_metrics_ignore_updates() {
        let mut metrics = PerfMetrics::new(false);
        let core = NesCore::new();
        let frame = RgbaFrame::default();
        metrics.on_step(&core, Duration::from_millis(5), false);
        metrics.on_render(&frame, Duration::from_millis(5));
        metrics.on_audio_queue(5, true);
        metrics.on_netplay_stats(&NetplayRuntimeStats::new(4));

        assert_eq!(metrics.report_frames, 0);
        assert_eq!(metrics.audio_queue_peak, 0);
        assert_eq!(metrics.audio_queue_drops, 0);
        assert_eq!(metrics.step_work, Duration::ZERO);
        assert_eq!(metrics.render_work, Duration::ZERO);
    }
"""

content = content.replace("fn disabled_metrics_stay_silent() {", test_func + "\n    #[test]\n    fn disabled_metrics_stay_silent() {")


with open("crates/nes-desktop/src/metrics.rs", "w") as f:
    f.write(content)
