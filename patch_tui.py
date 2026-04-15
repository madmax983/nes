import re

with open("crates/nes-tui/src/render.rs", "r") as f:
    content = f.read()

old_code = """#[must_use]
fn choose_quarter_mask_and_palette(samples: [(u8, u8, u8); 4]) -> (u8, (u8, u8, u8), (u8, u8, u8)) {
    let mut candidates = Vec::with_capacity(4);
    for sample in samples {
        if !candidates.contains(&sample) {
            candidates.push(sample);
        }
    }
    if candidates.is_empty() {
        return (0, (0, 0, 0), (0, 0, 0));
    }
    if candidates.len() == 1 {
        return (15, candidates[0], candidates[0]);
    }

    let mut best_error = u32::MAX;
    let mut best = (15_u8, candidates[0], candidates[1]);

    for &fg in &candidates {
        for &bg in &candidates {"""

new_code = """#[must_use]
fn choose_quarter_mask_and_palette(samples: [(u8, u8, u8); 4]) -> (u8, (u8, u8, u8), (u8, u8, u8)) {
    let mut candidates = [(0, 0, 0); 4];
    let mut len = 0;
    for sample in samples {
        if !candidates[..len].contains(&sample) {
            candidates[len] = sample;
            len += 1;
        }
    }
    let candidates = &candidates[..len];

    if candidates.is_empty() {
        return (0, (0, 0, 0), (0, 0, 0));
    }
    if candidates.len() == 1 {
        return (15, candidates[0], candidates[0]);
    }

    let mut best_error = u32::MAX;
    let mut best = (15_u8, candidates[0], candidates[1]);

    for &fg in candidates {
        for &bg in candidates {"""

content = content.replace(old_code, new_code)

with open("crates/nes-tui/src/render.rs", "w") as f:
    f.write(content)
