#![cfg(feature = "nova")]

use crate::NesCore;

/// An interactive frame-by-frame debugger and memory viewer utility.
pub struct HomebrewDebugger {
    pub memory_page: u16,
}

impl Default for HomebrewDebugger {
    fn default() -> Self {
        Self::new()
    }
}

impl HomebrewDebugger {
    #[must_use]
    pub fn new() -> Self {
        Self { memory_page: 0 }
    }

    /// Formats the current CPU registers as a string.
    #[must_use]
    pub fn format_registers(core: &NesCore) -> String {
        let snapshot = core.cpu_snapshot();
        format!(
            "PC:${:04X} A:${:02X} X:${:02X} Y:${:02X} SP:${:02X} P:%{:08b}",
            snapshot.pc, snapshot.a, snapshot.x, snapshot.y, snapshot.sp, snapshot.status
        )
    }

    /// Formats a hex dump of the currently selected memory page (256 bytes).
    #[must_use]
    pub fn format_memory_hex(&self, core: &NesCore) -> Vec<String> {
        let mut lines = Vec::with_capacity(16);
        let base_addr = self.memory_page * 256;
        for row in 0..16 {
            let row_addr = base_addr + (row * 16);
            let mut line = format!("${:04X}:", row_addr);
            for col in 0..16 {
                let addr = row_addr + col;
                line.push_str(&format!(" {:02X}", core.read_memory(addr)));
            }
            lines.push(line);
        }
        lines
    }

    /// Returns the last CPU trace instruction if available.
    #[must_use]
    pub fn format_disassembly(core: &NesCore) -> String {
        core.last_cpu_trace()
            .unwrap_or("No trace available (trace disabled or CPU not stepped)")
            .to_string()
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;
    use crate::NesCore;

    #[test]
    fn test_debugger_formats_registers() {
        let core = NesCore::new();
        let regs = HomebrewDebugger::format_registers(&core);
        assert!(regs.contains("PC:$"));
    }

    #[test]
    fn test_debugger_formats_memory() {
        let core = NesCore::new();
        let debugger = HomebrewDebugger::new();
        let mem = debugger.format_memory_hex(&core);
        assert_eq!(mem.len(), 16);
        assert!(mem[0].starts_with("$0000:"));
    }
}
