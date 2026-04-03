//! 6502 CPU implementation and status register helpers.

mod engine;
/// Processor status register (`P`) helper type.
pub mod status;

pub use engine::{
    Cpu, CpuBusAccess, CpuBusAccessKind, CpuError, CpuMmioRead, CpuPrgWrite, CpuSnapshot, CpuWrite,
};
