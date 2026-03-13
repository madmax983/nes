use loom::thread;
use nes_core::NesCore;
use nes_mcp::{dispatch_tool, ToolParams};

#[test]
fn havoc_loom_save_state_contention() {
    loom::model(|| {
        let mut core = NesCore::new();
        let mut core2 = NesCore::new();
        let mut params1 = ToolParams::new();
        params1.insert("slot".to_owned(), "slot1".to_owned());
        let mut params2 = ToolParams::new();
        params2.insert("slot".to_owned(), "slot1".to_owned());

        let t1 = thread::spawn(move || {
            let _ = dispatch_tool(&mut core, "save_state", &params1);
        });

        let t2 = thread::spawn(move || {
            let _ = dispatch_tool(&mut core2, "load_state", &params2);
        });

        t1.join().unwrap();
        t2.join().unwrap();
    });
}
