## 2026-08-11 - Flag-Dependent CPU Instruction Testing
**Learning:** Testing flag-dependent CPU instructions like `ADC` and `ROR` in `nes-core` requires careful setup to inject the initial status flag state (e.g., the Carry flag). The existing testing macro/struct only mocked data registers (`A`, `X`, `Y`), leaving flags undefined.
**Action:** Extend the generic `TestCase` struct and setup logic to explicitly control required status flags (like `setup_c`) and inject the corresponding prep instructions (`SEC`/`CLC`) before the target instruction.
