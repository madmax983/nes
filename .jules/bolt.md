## 2025-06-19 - Removed format! heap allocation on hot IO path

**Learning:** When writing formatted strings (like headers) to an IO writer (like `TcpStream` or `Stdout`), using `writer.write_all(format!("...").as_bytes())` causes an unnecessary intermediate `String` allocation on the heap for every write.
**Action:** Always use the `write!` macro (e.g., `write!(writer, "...", ...)`) from `std::io::Write` instead. It writes directly to the stream without allocating an intermediate string buffer.
