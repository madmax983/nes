//! Experimental CPU execution graph generator.
//!
//! Tracks the sequence of Program Counter (PC) values to build a directed graph of
//! CPU execution flow. It can export this flow to Graphviz DOT format, making it
//! incredibly useful for reverse engineering NES game loops or debugging AI agents.

#[cfg(feature = "nova")]
use crate::NesCore;
#[cfg(feature = "nova")]
use std::collections::{HashMap, HashSet};

#[cfg(feature = "nova")]
#[derive(Debug, Default, Clone)]
/// Records the flow of CPU execution by tracking Program Counter (PC) transitions.
///
/// `ExecutionGraph` builds a directed graph representing the paths the CPU takes during
/// execution. This is invaluable for reverse engineering ROMs, discovering hidden game
/// loops, or visualizing where an AI agent spends its time.
///
/// ## Examples
///
/// ```
/// # use nes_core::NesCore;
/// # use nes_core::experimental::execution_graph::ExecutionGraph;
/// let mut core = NesCore::new();
/// let mut graph = ExecutionGraph::new();
///
/// // Simulate stepping the CPU and recording the transition
/// graph.record_step(&core);
/// let _ = core.execute(nes_core::Command::StepCpu);
/// graph.record_step(&core);
///
/// // Export the recorded path to Graphviz DOT format
/// let dot_data = graph.to_dot();
/// assert!(dot_data.contains("digraph ExecutionFlow {"));
/// ```
pub struct ExecutionGraph {
    /// Maps a source PC to a set of destination PCs.
    transitions: HashMap<u16, HashSet<u16>>,
    last_pc: Option<u16>,
}

#[cfg(feature = "nova")]
impl ExecutionGraph {
    /// Creates a new, empty execution graph.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Records the current PC from the core, creating a transition from the last recorded PC.
    /// This should be called after each CPU step.
    pub fn record_step(&mut self, core: &NesCore) {
        let current_pc = core.cpu_pc();
        if let Some(last) = self.last_pc {
            // Only record jumps/branches or sequential execution
            self.transitions.entry(last).or_default().insert(current_pc);
        }
        self.last_pc = Some(current_pc);
    }

    /// Exports the recorded transitions to a Graphviz DOT string.
    #[must_use]
    /// **Performance optimization:** Avoids intermediate `String` allocations by using `writeln!`.
    pub fn to_dot(&self) -> String {
        use std::fmt::Write;
        let mut dot = String::from("digraph ExecutionFlow {\n");
        // Sort keys to ensure deterministic output for testing/version control
        let mut srcs: Vec<_> = self.transitions.keys().copied().collect();
        srcs.sort_unstable();

        for src in srcs {
            let mut dests: Vec<_> = self.transitions[&src].iter().copied().collect();
            dests.sort_unstable();
            for dest in dests {
                let _ = writeln!(dot, "    \"{:04X}\" -> \"{:04X}\";", src, dest);
            }
        }
        dot.push_str("}\n");
        dot
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;

    #[test]
    fn should_record_transitions_and_export_dot() {
        let mut core = NesCore::new();
        // Force the PC to start at 0xC000
        core.load_cpu_bytes(0xC000, &[0xEA, 0xEA, 0x4C, 0x00, 0xC0]); // NOP, NOP, JMP $C000

        let mut graph = ExecutionGraph::new();

        // Initial PC
        graph.record_step(&core);

        // Execute NOP
        let _ = core.execute(crate::Command::StepCpu);
        graph.record_step(&core);

        // Execute NOP
        let _ = core.execute(crate::Command::StepCpu);
        graph.record_step(&core);

        // Execute JMP
        let _ = core.execute(crate::Command::StepCpu);
        graph.record_step(&core);

        let dot = graph.to_dot();

        assert!(dot.contains("\"C000\" -> \"C001\""));
        assert!(dot.contains("\"C001\" -> \"C002\""));
        assert!(dot.contains("\"C002\" -> \"C000\""));
        assert!(dot.starts_with("digraph ExecutionFlow {"));
        assert!(dot.ends_with("}\n"));
    }

    #[test]
    fn should_export_multiple_transitions_efficiently() {
        let mut graph = ExecutionGraph::new();
        graph.last_pc = Some(0x8000);
        graph.transitions.entry(0x8000).or_default().insert(0x8001);
        graph.transitions.entry(0x8000).or_default().insert(0x8005);
        let dot = graph.to_dot();
        assert!(dot.contains("    \"8000\" -> \"8001\";\n"));
        assert!(dot.contains("    \"8000\" -> \"8005\";\n"));
    }
}
