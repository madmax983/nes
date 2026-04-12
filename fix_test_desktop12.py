import re

with open("crates/nes-desktop/src/metrics.rs", "r") as f:
    content = f.read()

# remove duplicate test attribute
content = re.sub(r'    #\[test\]\n    #\[test\]\n', '    #[test]\n', content)
content = content.replace("assert_eq!(snapshot.step_ms, 0.0);", "assert_eq!(snapshot.avg_step_ms, 0.0);")
content = content.replace("assert_eq!(snapshot.render_ms, 0.0);", "assert_eq!(snapshot.avg_render_ms, 0.0);")


with open("crates/nes-desktop/src/metrics.rs", "w") as f:
    f.write(content)
