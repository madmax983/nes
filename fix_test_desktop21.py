import re

with open("crates/nes-desktop/src/metrics.rs", "r") as f:
    content = f.read()

test_func = """
    #[test]
    fn perf_metrics_on_step_handles_initial_pc() {
        let core = NesCore::new();
        let mut metrics = PerfMetrics::new(true, 0, 0);

        // Before first step, last_pc is None
        assert_eq!(metrics.last_pc, None);

        metrics.on_step(&core, Duration::from_millis(4), false);

        // last_pc should be populated now, but stall count should be 0 because it didn't match None
        assert_eq!(metrics.pc_stall_frames, 0);
        assert_eq!(metrics.last_pc, Some(core.cpu_pc()));
    }
"""

content = content.replace("fn perf_metrics_on_step_tracks_stalls_and_recovers_on_pc_change() {", test_func + "\n    #[test]\n    fn perf_metrics_on_step_tracks_stalls_and_recovers_on_pc_change() {")

with open("crates/nes-desktop/src/metrics.rs", "w") as f:
    f.write(content)
