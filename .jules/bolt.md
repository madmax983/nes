## [Performance] Removed heap allocation in audio sample generation
**Learning:** Intermediate buffers with `Vec::with_capacity` in public-facing apis (like `apu::drain_samples`) forces a heap allocation on every single frame. This was found in `audio_chunk_i16`.
**Action:** Switch public APIs that yield buffers (like audio samples) to accept a `&mut [T]` to fill, pushing the allocation decision to the caller and removing intermediate allocations entirely.
