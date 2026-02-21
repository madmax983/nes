use std::env;

#[test]
#[ignore = "requires BLARGG_CPU_ROM_PATH"]
fn blargg_cpu_rom_smoke_gate() {
    let rom_path =
        env::var("BLARGG_CPU_ROM_PATH").expect("set BLARGG_CPU_ROM_PATH to run this test");
    assert!(
        !rom_path.trim().is_empty(),
        "BLARGG_CPU_ROM_PATH cannot be empty"
    );
}
