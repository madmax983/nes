import re

with open("crates/nes-desktop/src/metrics.rs", "r") as f:
    content = f.read()

test_func = """
    #[test]
    fn perf_metrics_on_render_resets_unchanged_frame_count() {
        let mut metrics = PerfMetrics::new(true, 0, 0);
        let mut frame = vec![0_u8; 256 * 240 * 4];

        metrics.on_render(&frame, Duration::from_millis(3));
        assert_eq!(metrics.unchanged_frame_count, 0);

        metrics.on_render(&frame, Duration::from_millis(2));
        assert_eq!(metrics.unchanged_frame_count, 1);

        frame[0] = 1;
        metrics.on_render(&frame, Duration::from_millis(2));
        assert_eq!(metrics.unchanged_frame_count, 0);
    }
"""

content = content.replace("fn perf_metrics_render_audio_and_netplay_observation_update_fields() {", test_func + "\n    #[test]\n    fn perf_metrics_render_audio_and_netplay_observation_update_fields() {")

with open("crates/nes-desktop/src/metrics.rs", "w") as f:
    f.write(content)
