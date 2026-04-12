import re

with open("crates/nes-desktop/src/metrics.rs", "r") as f:
    content = f.read()

test_func = """
    #[test]
    fn compute_metrics_snapshot_handles_saturating_ppu_delta() {
        let snapshot = compute_metrics_snapshot(
            1,
            1.0,
            200,
            100, // less than start, will saturating sub to 0
            Duration::from_millis(3),
            Duration::from_millis(2),
        )
        .expect("valid input should produce snapshot");
        assert_eq!(snapshot.emu_fps, 0.0);
    }
"""

content = content.replace("fn compute_metrics_snapshot_handles_guard_conditions_and_saturating_ppu_delta() {", test_func + "\n    #[test]\n    fn compute_metrics_snapshot_handles_guard_conditions_and_saturating_ppu_delta() {")

with open("crates/nes-desktop/src/metrics.rs", "w") as f:
    f.write(content)
