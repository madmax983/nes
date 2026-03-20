use nes_core::bmp::encode_bmp;
use proptest::prelude::*;

proptest! {
    #[test]
    #[ignore = "havoc target"]
    fn havoc_encode_bmp_panics_on_small_buffer(
        width in 1usize..100,
        height in 1usize..100,
        rgba in any::<Vec<u8>>().prop_filter("buffer too small", |v| v.len() < 4)
    ) {
        let _ = encode_bmp(width, height, &rgba);
    }
}
