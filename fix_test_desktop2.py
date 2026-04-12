import re

with open("crates/nes-desktop/src/metrics.rs", "r") as f:
    content = f.read()

# remove duplicate test attribute
content = re.sub(r'    #\[test\]\n    #\[test\]\n    fn frame_signature_ignores_unsampled_bytes\(\) \{', '    #[test]\n    fn frame_signature_ignores_unsampled_bytes() {', content)
content = content.replace("let mut frame_a = vec![0_u8; 256];", "let frame_a = vec![0_u8; 256];")

with open("crates/nes-desktop/src/metrics.rs", "w") as f:
    f.write(content)
