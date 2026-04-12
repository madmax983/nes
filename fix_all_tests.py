import re
import glob

files = glob.glob("crates/**/*.rs", recursive=True)
for filepath in files:
    with open(filepath, "r") as f:
        content = f.read()

    # aggressively remove double test attributes
    while '#[test]\n    #[test]' in content or '#[test]\n#[test]' in content:
        content = re.sub(r'    #\[test\]\n    #\[test\]', '    #[test]', content)
        content = re.sub(r'#\[test\]\n#\[test\]', '#[test]', content)

    # specifically for nes-desktop metrics.rs spacing anomalies
    content = re.sub(r'#\[test\]\n\s*#\[test\]', '#[test]', content)

    with open(filepath, "w") as f:
        f.write(content)
