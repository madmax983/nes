content = open('crates/nes-dsl/src/lib.rs').read()

import re

# find the function parse_number_literal inside #[cfg(test)] and move it out
search_regex = r'#\[cfg\(test\)\]\nfn parse_number_literal\(s: &str\) -> Option<i64> \{\n    if let Some\(hex\) = s\.strip_prefix\(\'\$\'\) \{\n        return i64::from_str_radix\(hex, 16\)\.ok\(\);\n    \}\n    if let Some\(bin\) = s\.strip_prefix\(\'%\'\) \{\n        return i64::from_str_radix\(bin, 2\)\.ok\(\);\n    \}\n    if let Some\(hex\) = s\.strip_prefix\(\"0x\"\)\.or_else\(\|\| s\.strip_prefix\(\"0X\"\)\) \{\n        return i64::from_str_radix\(hex, 16\)\.ok\(\);\n    \}\n    s\.parse::<i64>\(\)\.ok\(\)\n\}\n'

match = re.search(search_regex, content)
if match:
    func_text = match.group(0).replace("#[cfg(test)]\n", "")
    content = content.replace(match.group(0), "")
    test_mod_index = content.find("mod tests {")
    # before test_mod_index, there's likely a #[cfg(test)] tag
    cfg_test_index = content.rfind("#[cfg(test)]", 0, test_mod_index)
    if cfg_test_index != -1:
        insert_index = cfg_test_index
    else:
        insert_index = test_mod_index

    new_content = content[:insert_index] + func_text + "\n" + content[insert_index:]
    open('crates/nes-dsl/src/lib.rs', 'w').write(new_content)
