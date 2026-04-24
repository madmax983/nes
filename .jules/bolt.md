**[Allocating inside loops]
**Learning:** Instantiating objects like `String::new()` inside tight network read loops (like receiving 60 frames per second over TCP) causes massive unnecessary heap allocations. Using `.clear()` on a hoisted buffer avoids this completely.
**Action:** Always hoist `String` and `Vec` buffers out of hot loops when iterating over I/O streams using `read_line` or similar methods.
