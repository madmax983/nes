content = open('crates/nes-dsl/src/lib.rs').read()

import re

test_mod_index = content.find("mod tests {")
if test_mod_index != -1:
    test_func = """
    #[test]
    fn parse_number_literal_handles_valid_formats() {
        assert_eq!(parse_number_literal("$1A"), Some(26));
        assert_eq!(parse_number_literal("%1101"), Some(13));
        assert_eq!(parse_number_literal("0x1A"), Some(26));
        assert_eq!(parse_number_literal("0X1A"), Some(26));
        assert_eq!(parse_number_literal("42"), Some(42));
    }

    #[test]
    fn parse_number_literal_handles_invalid_formats() {
        assert_eq!(parse_number_literal("$ZZ"), None);
        assert_eq!(parse_number_literal("%12"), None);
        assert_eq!(parse_number_literal("0xZZ"), None);
        assert_eq!(parse_number_literal("not_a_number"), None);
    }
"""
    # insert tests just inside the mod tests { block
    insert_pos = test_mod_index + len("mod tests {")
    new_content = content[:insert_pos] + "\n" + test_func + content[insert_pos:]
    open('crates/nes-dsl/src/lib.rs', 'w').write(new_content)
