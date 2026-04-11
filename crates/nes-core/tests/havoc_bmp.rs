use proptest::prelude::*;
use nes_core::bmp::encode_bmp;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(5000))]
    #[test]
    #[ignore = "havoc target"]
    fn havoc_fuzz_bmp_encoder(
        width in 0usize..100,
        height in 0usize..100,
        rgba in proptest::collection::vec(any::<u8>(), 0..40000),
    ) {
        // The trigger: The `encode_bmp` function assumes `rgba.len() >= width * height * 4`.
        // If we pass a smaller array, it will panic inside the pixel extraction loop when
        // calculating `idx + 2`.
        let _ = encode_bmp(width, height, &rgba);
    }
}
