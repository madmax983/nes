Wait, `choose_quarter_mask_and_palette` allocates a `Vec` for `candidates`!
```rust
#[must_use]
fn choose_quarter_mask_and_palette(samples: [(u8, u8, u8); 4]) -> (u8, (u8, u8, u8), (u8, u8, u8)) {
    let mut candidates = Vec::with_capacity(4);
    for sample in samples {
        if !candidates.contains(&sample) {
            candidates.push(sample);
        }
    }
```
This is called per cell in `frame_lines_quarter_blocks`!
```rust
        for col in 0..width {
            ...
            let samples = [ ... ];
            let (mask, fg, bg) = choose_quarter_mask_and_palette(samples);
```
If the terminal is 80x40, that's 3200 `Vec::with_capacity(4)` allocations *per frame*! 60 times a second! That's 192,000 heap allocations per second on the rendering hot path! This is a massive bottleneck.
And it only ever holds up to 4 elements.

I can optimize this by replacing `Vec::with_capacity(4)` with a stack-allocated array (or `arrayvec` / `tinyvec`, but a simple `[Option<(u8, u8, u8)>; 4]` or just keeping track of `len` in a fixed `[ (0,0,0); 4 ]` array works safely with 0 allocations).

Let's test this in `nes-tui/src/render.rs`.
