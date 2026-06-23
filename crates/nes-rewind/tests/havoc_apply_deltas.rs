use nes_rewind::delta::{ArrayDelta, apply_deltas};
use proptest::prelude::*;
use smallvec::SmallVec;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(5000))]
    #[test]
    #[should_panic(expected = "out of range")]
    #[ignore = "havoc target"]
    fn havoc_fuzz_apply_deltas(
        mut target in proptest::collection::vec(any::<u8>(), 0..1024),
        deltas_bytes in proptest::collection::vec(any::<u8>(), 0..1024),
    ) {
        let mut deltas = Vec::new();
        // Construct some random deltas
        let chunk_iter = deltas_bytes.chunks(5);
        for chunk in chunk_iter {
            if chunk.len() == 5 {
                let offset = u16::from_le_bytes([chunk[0], chunk[1]]);
                let mut data = SmallVec::new();
                data.push(chunk[2]);
                data.push(chunk[3]);
                data.push(chunk[4]);
                deltas.push(ArrayDelta { offset, data });
            }
        }

        apply_deltas(&mut target, &deltas);
    }
}
