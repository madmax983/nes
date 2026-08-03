import re
with open('crates/nes-dsl/src/parser.rs', 'r') as f:
    content = f.read()

new_func = """/// **Performance optimization:** Avoids `.collect::<Vec<_>>()` by using an iterator,
/// and uses `&str` instead of returning a `String` to eliminate O(N) heap
/// allocations per line of DSL code parsed, taking slices directly from the input.
pub(crate) fn strip_comments(line: &str) -> &str {
    let mut in_string = false;
    let mut escaped = false;

    for (idx, ch) in line.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\\\\\\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        if ch == '"' {
            in_string = true;
            continue;
        }
        if ch == ';' {
            return &line[..idx];
        }
        if ch == '/' {
            if line[idx..].starts_with("//") {
                return &line[..idx];
            }
        }
    }
    line
}"""

content = re.sub(r'/// \*\*Performance optimization:.*?\n.*?\npub\(crate\) fn strip_comments.*?^}', new_func, content, flags=re.MULTILINE|re.DOTALL)
with open('crates/nes-dsl/src/parser.rs', 'w') as f:
    f.write(content)

with open('crates/nes-dsl/src/lib.rs', 'r') as f:
    lib_content = f.read()

new_func_lib = """/// **Performance optimization:** Avoids `.collect::<Vec<_>>()` by using an iterator,
/// and uses `&str` instead of returning a `String` to eliminate O(N) heap
/// allocations per line of DSL code parsed, taking slices directly from the input.
fn strip_comments(line: &str) -> &str {
    let mut in_string = false;
    let mut escaped = false;

    for (idx, ch) in line.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\\\\\\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        if ch == '"' {
            in_string = true;
            continue;
        }
        if ch == ';' {
            return &line[..idx];
        }
        if ch == '/' {
            if line[idx..].starts_with("//") {
                return &line[..idx];
            }
        }
    }
    line
}"""

lib_content = re.sub(r'/// \*\*Performance optimization:.*?\n.*?\nfn strip_comments.*?^}', new_func_lib, lib_content, flags=re.MULTILINE|re.DOTALL)
with open('crates/nes-dsl/src/lib.rs', 'w') as f:
    f.write(lib_content)
