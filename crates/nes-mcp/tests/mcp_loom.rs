use loom::thread;

// Testing the thread-safe dispatch architecture indirectly via the public tools map.
use nes_mcp::tools::tool_catalog;

#[test]
fn loom_tool_catalog_concurrent_access() {
    loom::model(|| {
        let t1 = thread::spawn(move || {
            let _ = tool_catalog();
        });

        let t2 = thread::spawn(move || {
            let _ = tool_catalog();
        });

        t1.join().unwrap();
        t2.join().unwrap();
        let _ = tool_catalog();
    });
}
