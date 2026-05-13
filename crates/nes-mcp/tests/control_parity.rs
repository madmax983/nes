use nes_core::{Command, NesCore};
use nes_mcp::{ToolParams, dispatch_tool};

#[test]
fn pause_tool_matches_direct_core_command() {
    let mut via_core = NesCore::new();
    via_core.execute(Command::Pause).expect("valid test data");

    let mut via_mcp = NesCore::new();
    dispatch_tool(&mut via_mcp, "pause", &ToolParams::new()).expect("valid test data");

    assert_eq!(via_core.state_hash(), via_mcp.state_hash());
}
