content = open('crates/nes-dsl/src/lib.rs').read()

search = """    let parsed = if let Some(hex) = rest.strip_prefix('$') {
        i64::from_str_radix(hex, 16).ok()
    } else if let Some(bin) = rest.strip_prefix('%') {
        i64::from_str_radix(bin, 2).ok()
    } else if let Some(hex) = rest.strip_prefix("0x").or_else(|| rest.strip_prefix("0X")) {
        i64::from_str_radix(hex, 16).ok()
    } else {
        rest.parse::<i64>().ok()
    };"""

replace = """    let parsed = parse_number_literal(rest);"""

content = content.replace(search, replace, 1)

new_func = """
fn parse_number_literal(s: &str) -> Option<i64> {
    if let Some(hex) = s.strip_prefix('$') {
        return i64::from_str_radix(hex, 16).ok();
    }
    if let Some(bin) = s.strip_prefix('%') {
        return i64::from_str_radix(bin, 2).ok();
    }
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        return i64::from_str_radix(hex, 16).ok();
    }
    s.parse::<i64>().ok()
}
"""
content += new_func
open('crates/nes-dsl/src/lib.rs', 'w').write(content)
