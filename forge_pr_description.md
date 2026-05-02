🚮 Smell: The `advance_hardware_cycles` and `apply_dmc_dma_request` methods in `crates/nes-core/src/api.rs` both duplicated the exact same nested loop logic for advancing the PPU dot 3 times per APU/CPU cycle and applying mapper IRQ checks.

✨ Solution: Extracted this inner hardware synchronization loop into a shared helper `step_hardware_cycle`, flattening the DMA and batch-cycle loops and completely eliminating the duplicated mapper dot logic.

🧼 Benefit: Dramatically improves readability, strictly adheres to DRY without adding overhead, and reduces the chance of future DMA stall bugs going out of sync with normal execution.

🛡️ Verification: Tests passed. No logic changed.
