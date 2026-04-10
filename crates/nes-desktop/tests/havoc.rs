#[test]
#[ignore]
fn havoc_ppm_oom_panic() {
    // Intentionally trigger an Out of Memory (OOM) panic via a massive vector allocation.
    // This demonstrates the fragility of assuming memory is always available,
    // particularly concerning unchecked `.unwrap()` usages on memory buffer `write!`
    // macros in the system (e.g., `encode_ppm` in `main.rs`).
    let mut ppm = Vec::with_capacity(usize::MAX);
    use std::io::Write;
    let _ = write!(&mut ppm, "P6\n{} {}\n255\n", 256, 240).unwrap();
}
