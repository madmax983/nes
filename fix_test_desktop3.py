import re

with open("crates/nes-desktop/src/metrics.rs", "r") as f:
    content = f.read()

test_func = """
    #[test]
    fn compute_metrics_snapshot_returns_none_when_frames_is_zero() {
        assert!(compute_metrics_snapshot(0, 1.0, 10, 20, Duration::ZERO, Duration::ZERO).is_none());
    }

    #[test]
    fn compute_metrics_snapshot_returns_none_when_elapsed_is_zero() {
        assert!(compute_metrics_snapshot(1, 0.0, 10, 20, Duration::ZERO, Duration::ZERO).is_none());
        assert!(compute_metrics_snapshot(1, -1.0, 10, 20, Duration::ZERO, Duration::ZERO).is_none());
    }
"""

content = content.replace("fn compute_metrics_snapshot_handles_guard_conditions_and_saturating_ppu_delta() {", test_func + "\n    #[test]\n    fn compute_metrics_snapshot_handles_guard_conditions_and_saturating_ppu_delta() {")

with open("crates/nes-desktop/src/metrics.rs", "w") as f:
    f.write(content)
