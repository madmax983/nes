import re

with open('crates/nes-mcp/src/dispatch.rs', 'r') as f:
    content = f.read()

# I accidentally added the tests into the havoc_fuzz_tests module!
# Let's move them to the mod tests module.
tests = """
    #[test]
    fn test_parse_u8_validates_value_and_range() {
        assert_eq!(parse_u8(&params(&[("val", "42")]), "val").expect("valid u8"), 42);

        let missing = parse_u8(&ToolParams::new(), "val").expect_err("missing value");
        assert_eq!(missing.to_string(), "invalid params: val must be provided");

        let out_of_bounds = parse_u8(&params(&[("val", "256")]), "val").expect_err("out of bounds");
        assert_eq!(out_of_bounds.to_string(), "invalid params: val must be an integer in [0, 255]");

        let not_a_number = parse_u8(&params(&[("val", "abc")]), "val").expect_err("not a number");
        assert_eq!(not_a_number.to_string(), "invalid params: invalid integer literal 'abc'");
    }

    #[test]
    fn test_parse_u16_validates_value_and_range() {
        assert_eq!(parse_u16(&params(&[("val", "12345")]), "val").expect("valid u16"), 12345);

        let missing = parse_u16(&ToolParams::new(), "val").expect_err("missing value");
        assert_eq!(missing.to_string(), "invalid params: val must be provided");

        let out_of_bounds = parse_u16(&params(&[("val", "65536")]), "val").expect_err("out of bounds");
        assert_eq!(out_of_bounds.to_string(), "invalid params: val must be an integer in [0, 65535]");

        let not_a_number = parse_u16(&params(&[("val", "abc")]), "val").expect_err("not a number");
        assert_eq!(not_a_number.to_string(), "invalid params: invalid integer literal 'abc'");
    }

    #[test]
    fn test_parse_required_string_validates_presence_and_length() {
        assert_eq!(parse_required_string(&params(&[("val", "hello")]), "val").expect("valid string"), "hello");

        let missing = parse_required_string(&ToolParams::new(), "val").expect_err("missing value");
        assert_eq!(missing.to_string(), "invalid params: val must be provided");

        let empty = parse_required_string(&params(&[("val", "  ")]), "val").expect_err("empty string");
        assert_eq!(empty.to_string(), "invalid params: val must not be empty");
    }

    #[test]
    fn test_parse_dsl_source_validates_presence_and_length() {
        assert_eq!(parse_dsl_source(&params(&[("source", "LDA #$00")])).expect("valid source"), "LDA #$00");

        let missing = parse_dsl_source(&ToolParams::new()).expect_err("missing source");
        assert_eq!(missing.to_string(), "invalid params: source must be provided");

        let empty = parse_dsl_source(&params(&[("source", " \n\t ")])).expect_err("empty source");
        assert_eq!(empty.to_string(), "invalid params: source must not be empty");
    }

    #[test]
    fn test_parse_rom_payload_requires_path_or_hex() {
        let missing = parse_rom_payload(&ToolParams::new()).expect_err("missing path and hex");
        assert_eq!(missing.to_string(), "invalid params: provide rom_hex or rom_path");
    }
"""

start_idx = content.find("    #[test]\n    fn test_parse_u8_validates_value_and_range() {")
if start_idx != -1:
    content = content[:start_idx] + "}\n"

# Now append to `mod tests` block.
target_idx = content.find("#[cfg(test)]\nmod havoc_fuzz_tests {")
if target_idx != -1:
    # insert tests before target_idx
    # actually, target_idx is the start of the next mod. the closing brace of mod tests is just before it
    # find the brace before target_idx
    brace_idx = content.rfind("}", 0, target_idx)
    content = content[:brace_idx] + tests + "\n" + content[brace_idx:]

with open('crates/nes-mcp/src/dispatch.rs', 'w') as f:
    f.write(content)
