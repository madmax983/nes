# 👺 Havoc: Proptest fuzzing public string parsing APIs

## 🧨 The Trigger:
Input strings with unexpected characters or invalid shapes passed to public string parsing APIs.
Found missing property-based testing coverage for string parsing APIs across the system.

## 📉 The Stack Trace:
(Intentionally avoiding actual bug fixes, creating test harnesses to prove that edge cases are exercised.)

## 🧪 Reproduction:
Run `cargo test -p nes-dsl havoc_fuzz`
Run `cargo test -p nes-desktop havoc_fuzz`
Run `cargo test -p nes-mcp havoc_fuzz`

## 😈 Comment:
Added property testing harnesses for string parsing functions. Let the chaos flow!
