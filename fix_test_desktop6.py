import re

with open("crates/nes-desktop/src/metrics.rs", "r") as f:
    content = f.read()

# remove duplicate test attribute
content = re.sub(r'    #\[test\]\n    #\[test\]\n    fn compute_metrics_snapshot_handles_guard_conditions_and_saturating_ppu_delta\(\) \{', '    #[test]\n    fn compute_metrics_snapshot_handles_guard_conditions_and_saturating_ppu_delta() {', content)

with open("crates/nes-desktop/src/metrics.rs", "w") as f:
    f.write(content)
