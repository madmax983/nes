**[Defer Expensive Computations in Loop]**
**Learning:** For APU mixing in `nes-core`, calculating `raw_mixed_sample` inside the hot `step_cpu_cycle` is expensive and pointless to do if the audio buffer accumulator doesn't reach the threshold to consume the sample.
**Action:** Defer expensive per-sample computations like `raw_mixed_sample` inside rate-limiting conditional branches (e.g., `sample_accumulator >= CPU_CLOCK_HZ`) to prevent them from executing on every CPU cycle when not needed.
**[Defer Expensive Computations in Loop]**
**Learning:** For APU mixing in `nes-core`, calculating `raw_mixed_sample` inside the hot `step_cpu_cycle` is expensive and pointless to do if the audio buffer accumulator doesn't reach the threshold to consume the sample.
**Action:** Defer expensive per-sample computations like `raw_mixed_sample` inside rate-limiting conditional branches (e.g., `sample_accumulator >= CPU_CLOCK_HZ`) to prevent them from executing on every CPU cycle when not needed.
