import re

with open("crates/nes-core/src/experimental/hotspot_profiler.rs", "r") as f:
    content = f.read()

content = content.replace("hotspots.sort_unstable_by(|a, b| b.1.cmp(&a.1));", "hotspots.sort_unstable_by_key(|b| std::cmp::Reverse(b.1));")

with open("crates/nes-core/src/experimental/hotspot_profiler.rs", "w") as f:
    f.write(content)
