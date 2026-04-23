## 2023-10-24 - [Avoid unwrap_or clone on Option borrowed from HashMap]
**Learning:** [When dealing with an `Option<&Vec<T>>` returned from `HashMap::get`, using `.unwrap_or(&Vec::new()).clone()` forces an unnecessary heap allocation. Furthermore, `Vec` does not implement `map_or` directly, but `Option` does. Using `Option::is_none_or()` is the most idiomatic and performant way to evaluate a condition against the optional borrowed value without allocating.]
**Action:** [Always use `is_none_or` or `map_or` for conditional checks on borrowed `Option` values instead of cloning them or providing fallback heap allocations.]
**[Local variable doc comments]
**Learning:** [In Rust, applying a documentation comment (`///`) to a local statement (such as a `let` binding inside a function body) causes a compilation error (`error: expected outer doc comment`). Use standard comments (`//`) to document local variables and internal function logic.]
**Action:** [Always use `//` for comments inside functions, and reserve `///` for outer item declarations like functions, structs, and fields.]
**[Eliminating Header Buffer Allocations]
**Learning:** [When parsing line-based protocols (like MCP/JSON-RPC over stdio or TCP streams) in a loop, declaring `let mut line = String::new();` inside the loop causes a heap allocation for every single header line read.]
**Action:** [Hoist the `String` allocation outside the reading loop and call `line.clear()` before each `.read_line()` call. This allows `read_line` to safely reuse the existing buffer capacity, eliminating per-header-line allocations entirely.]

**Hoisting Strings out of IO Loops**
**Learning:** `BufRead::read_line` appends to the provided `String`. If you declare `let mut line = String::new();` inside a read loop, you will trigger a heap allocation for every line read from the network or file.
**Action:** Always hoist `String::new()` declarations outside of `loop` / `while` blocks that read streams, and call `.clear()` at the top of the loop body to reuse the existing buffer's capacity without allocating.
