import re

with open("crates/nes-desktop/src/metrics.rs", "r") as f:
    content = f.read()


test_func = """
    #[test]
    fn compute_metrics_snapshot_handles_zero_work() {
        let snapshot = compute_metrics_snapshot(
            120, // frames
            2.0, // seconds
            100, // start ppu
            220, // end ppu (diff 120)
            Duration::ZERO, // step work
            Duration::ZERO, // render work
        )
        .expect("valid input should produce snapshot");

        assert_eq!(snapshot.wall_fps, 60.0); // 120 / 2.0
        assert_eq!(snapshot.emu_fps, 60.0); // 120 / 2.0
        assert_eq!(snapshot.avg_step_ms, 0.0);
        assert_eq!(snapshot.avg_render_ms, 0.0);
    }
"""

content = content.replace("fn compute_metrics_snapshot_derives_expected_rates() {", test_func + "\n    #[test]\n    fn compute_metrics_snapshot_derives_expected_rates() {")

with open("crates/nes-desktop/src/metrics.rs", "w") as f:
    f.write(content)
