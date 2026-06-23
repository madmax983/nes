import sys

def patch_file(filepath):
    with open(filepath, 'r') as f:
        content = f.read()

    replacements = {
        "/// struct Dummy;": "/// // Used internally as `#[serde(with = \"crate::rta::serde_iter\")]` on struct fields.",
        "/// The human-readable name of the split (e.g., \"Defeated Bowser\").\n    pub name: String,": "    pub name: String,",
        "/// The exact emulator frame number when the split occurred.\n    pub frame: u64,": "    /// The exact emulator frame number when the split occurred.\n    pub frame: u64,",
        "/// The total elapsed wall-clock time in milliseconds since the run started.\n    pub elapsed_ms: u128,": "    /// The total elapsed wall-clock time in milliseconds since the run started.\n    pub elapsed_ms: u128,",
    }

    for k, v in replacements.items():
        content = content.replace(k, v)

    with open(filepath, 'w') as f:
        f.write(content)

patch_file('crates/nes-desktop/src/rta.rs')
