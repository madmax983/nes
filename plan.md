👺 Havoc: BMP Encoder Buffer Panic

🧨 **The Trigger:**
A call to `encode_bmp` where the length of the `rgba` byte array is less than `width * height * 4`.

📉 **The Stack Trace:**
```text
thread 'havoc_fuzz_bmp_encoder' panicked at crates/nes-core/src/bmp.rs:83:22:
index out of bounds: the len is 0 but the index is 6902
```

🧪 **Reproduction:**
Run `cargo test --test havoc_bmp havoc_fuzz_bmp_encoder -- --ignored`

😈 **Comment:**
"You didn't bounds-check your input array length against the width and height, trusting the caller unconditionally. I gave you 0 bytes of RAM to encode an image, and you blindly tried to read from index 6902. If I can crash it, I win."
