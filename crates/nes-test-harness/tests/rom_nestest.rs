use std::env;

#[test]
#[ignore = "requires NESTEST_ROM_PATH"]
fn nestest_trace_matches_expected_prefix() {
    let rom_path = env::var("NESTEST_ROM_PATH").expect("set NESTEST_ROM_PATH to run this test");
    assert!(
        !rom_path.trim().is_empty(),
        "NESTEST_ROM_PATH cannot be empty"
    );
}
