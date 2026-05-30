use nes_core::NesCore;
use nes_mcp::{ToolParams, dispatch_tool};

#[test]
#[ignore = "Havoc OOM Attack"]
fn havoc_test_save_state_oom() {
    // The `saved_states` is a static Mutex<HashMap<String, CoreSnapshot>>.
    // If an attacker repeatedly calls `save_state` with unique long slot names,
    // they can consume unbounded memory, eventually causing an OOM crash.
    let mut core = NesCore::new();

    // Create 1 million save states. CoreSnapshot is fairly large (~16KB+ depending on RAM).
    // 1M * 16KB = 16GB, easily causing an OOM on most CI runners.
    for i in 0..1_000_000 {
        let mut params = ToolParams::new();
        params.insert("slot".to_owned(), format!("havoc_slot_{}", i));
        let _ = dispatch_tool(&mut core, "save_state", &params);
    }
}
