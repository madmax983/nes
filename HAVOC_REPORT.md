# 👺 Havoc: encode_bmp panics on small buffers

🧨 **The Trigger:** Calling `encode_bmp` with an empty or very small `rgba` buffer and valid width/height causes an out of bounds panic because the function assumes the buffer matches the provided dimensions and indexing fails.

📉 **The Stack Trace:**
```
thread 'havoc_encode_bmp_panics_on_small_buffer' (33182) panicked at crates/nes-core/src/bmp.rs:83:22:
index out of bounds: the len is 1 but the index is 602
```

🧪 **Reproduction:** Run `cargo test -p nes-core --test havoc_bmp -- --ignored`.

😈 **Comment:** You assumed the `rgba` buffer would magically always match `width * height * 4`. You didn't validate it before blindly indexing into it. You were wrong.
