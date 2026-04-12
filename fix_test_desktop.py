import re

with open("crates/nes-desktop/src/metrics.rs", "r") as f:
    content = f.read()

test_func = """
    #[test]
    fn frame_signature_ignores_unsampled_bytes() {
        let mut frame_a = vec![0_u8; 256];
        let mut frame_b = vec![0_u8; 256];

        // Modify a byte that is NOT at index 0 within a 64-byte chunk
        frame_b[1] = 42;
        frame_b[65] = 42;

        let signature_a = frame_signature(&frame_a);
        let signature_b = frame_signature(&frame_b);
        assert_eq!(signature_a, signature_b);
    }
"""

content = content.replace("fn frame_signature_matches_reference_and_changes_on_sampled_byte() {", test_func + "\n    #[test]\n    fn frame_signature_matches_reference_and_changes_on_sampled_byte() {")


with open("crates/nes-desktop/src/metrics.rs", "w") as f:
    f.write(content)
