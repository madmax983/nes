use nes_core::{Command, NesCore};
use nes_mcp::{ToolParams, dispatch_tool};

#[test]
fn step_cpu_tool_matches_direct_core_execution() {
    let mut via_core = NesCore::new();
    via_core.load_cpu_bytes(0xC000, &[0xA9, 0x01]);
    via_core.execute(Command::StepCpu).unwrap();

    let mut via_mcp = NesCore::new();
    via_mcp.load_cpu_bytes(0xC000, &[0xA9, 0x01]);
    dispatch_tool(&mut via_mcp, "step_cpu", &ToolParams::new()).unwrap();

    assert_eq!(via_core.cpu_pc(), via_mcp.cpu_pc());
    assert_eq!(via_core.cpu_a(), via_mcp.cpu_a());
    assert_eq!(via_core.last_cpu_trace(), via_mcp.last_cpu_trace());
    assert_eq!(via_core.state_hash(), via_mcp.state_hash());
}
