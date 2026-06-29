**[Eliminate Redundant format! Allocations]**
**Learning:** Found a loop using `output.push_str(&format!("{byte:02X}"))` which creates an intermediate `String` allocation for every byte.
**Action:** Use `std::fmt::Write` and `write!(output, "{byte:02X}").unwrap()` to write directly into the pre-allocated string, avoiding redundant heap allocations.

**[False Optimizations]**
**Learning:** Tried to replace `.collect::<Vec<_>>()` with `Vec::with_capacity()` and `.extend()` on exact size iterators. Rust's `FromIterator` for `Vec` already pre-allocates perfectly if `size_hint()` is exact.
**Action:** Do not unroll or manually extend `collect::<Vec<_>>()` on exact size iterators. It is not an optimization and degrades idiomatic Rust code.

**[Struct Clone Allocations]**
**Learning:** Tried to optimize an RTA split event by returning `event.clone()` instead of constructing the struct twice with `name.clone()`.
**Action:** Realized that cloning a struct containing a `String` intrinsically clones the `String`, resulting in the exact same allocation overhead. Don't disguise logic tweaks as memory optimizations.
