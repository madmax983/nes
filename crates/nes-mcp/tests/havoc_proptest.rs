use nes_core::NesCore;
use nes_mcp::macro_engine::execute_macro_script;
use proptest::prelude::*;

proptest! {
    #[test]
    #[ignore = "Havoc Input Attack"]
    #[should_panic]
    fn havoc_test_macro_overflow(frames in any::<u64>()) {
        let mut core = NesCore::new();

        let script = format!("WAIT {}", frames);

        prop_assume!(frames > u64::MAX - 10);

        let _ = execute_macro_script(&mut core, &script, None).unwrap();
    }
}
