use nes_core::{Command, NesCore};
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_cpu_cycles_dont_overflow(ticks in 0..1000u64) {
        let mut core = NesCore::new();
        // Just ticking it forward a bunch of times shouldn't break internal state mathematically.
        for _ in 0..ticks {
            let _ = core.execute(Command::StepCpu);
        }
        // At least we didn't panic.
    }

    #[test]
    fn test_core_read_memory_fuzz(addr in 0..u16::MAX) {
        let core = NesCore::new();
        let _ = core.read_memory(addr);
    }
}
