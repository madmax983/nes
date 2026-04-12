import re

with open("crates/nes-desktop/src/metrics.rs", "r") as f:
    content = f.read()

# remove duplicate test attribute
content = re.sub(r'    #\[test\]\n    #\[test\]\n', '    #[test]\n', content)
content = re.sub(r'    #\[test\]\n    #\[test\]\n', '    #[test]\n', content)
content = re.sub(r'    #\[test\]\n    #\[test\]\n', '    #[test]\n', content)

with open("crates/nes-desktop/src/metrics.rs", "w") as f:
    f.write(content)
