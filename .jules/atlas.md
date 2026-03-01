**[Facade Pattern for CPU Engine]**
**Tangle:** The `cpu` module was public (`pub mod cpu`), leaking raw implementation details and forcing external consumers (like tests) to dig into nested paths (`nes_core::cpu::CpuBusAccessKind`). This violated strict module boundaries and exposed internals.
**Blueprint:** Converted `cpu` module to `pub(crate)` and exposed a clean flat API (`pub use api::{Cpu...}`) at the crate root (`lib.rs`). This establishes a strong "Facade" pattern, decoupling external usage from the internal module structure.
