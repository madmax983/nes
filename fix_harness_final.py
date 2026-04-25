import re

with open('crates/nes-test-harness/src/lib.rs', 'r') as f:
    content = f.read()

content = content.replace(
    'pub fn hann_window(idx: usize, len: usize) -> f64 {',
    '/// Applies a Hann window function to smooth out boundaries for spectral analysis.\n#[must_use]\npub fn hann_window(idx: usize, len: usize) -> f64 {'
)

with open('crates/nes-test-harness/src/lib.rs', 'w') as f:
    f.write(content)
