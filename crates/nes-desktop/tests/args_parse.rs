#[test]
fn parse_runtime_args_loops_multiple_args() {
    let args = vec![
        "--mcp-host".to_owned(),
        "--netplay".to_owned(),
        "game.nes".to_owned(),
    ];
    let parsed = nes_desktop::args::parse_runtime_args(&args).unwrap();
    assert!(parsed.mcp_enabled);
    assert!(parsed.netplay_enabled);
    assert_eq!(parsed.rom_path.as_deref(), Some("game.nes"));
}
