import re

with open("crates/nes-desktop/src/metrics.rs", "r") as f:
    content = f.read()

test_func = """
    #[test]
    fn compute_metrics_snapshot_handles_zero_work() {
        let snapshot = compute_metrics_snapshot(
            1,
            1.0,
            100,
            130,
            Duration::ZERO,
            Duration::ZERO,
        )
        .expect("valid input should produce snapshot");

        assert_eq!(snapshot.emu_fps, 30.0);
        assert_eq!(snapshot.step_ms, 0.0);
        assert_eq!(snapshot.render_ms, 0.0);
    }
"""

content = content.replace("fn compute_metrics_snapshot_handles_guard_conditions_and_saturating_ppu_delta() {", test_func + "\n    #[test]\n    fn compute_metrics_snapshot_handles_guard_conditions_and_saturating_ppu_delta() {")

with open("crates/nes-desktop/src/metrics.rs", "w") as f:
    f.write(content)
