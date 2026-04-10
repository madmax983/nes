import re

with open('crates/nes-desktop/src/main.rs', 'r') as f:
    main_content = f.read()

# We need to find the `mod tests` block and extract tests into config.rs
tests_block_re = re.compile(r'#\[cfg\(test\)\]\nmod tests \{.*?\n\}\n', re.DOTALL)
tests_match = tests_block_re.search(main_content)

if tests_match:
    tests_content = tests_match.group(0)

    config_tests = []

    config_funcs = ["map_virtual_keycode_maps_all_supported_keys"]

    for match in re.finditer(r'(#\[test\]\s+fn\s+(\w+)\(.*?\)\s*\{.*?^\s{4}\})', tests_content, re.DOTALL | re.MULTILINE):
        func_body = match.group(1)
        func_name = match.group(2)
        if func_name in config_funcs:
            config_tests.append(func_body)
            # Remove from main_content
            main_content = main_content.replace(func_body, "")

    # Write to config.rs
    with open('crates/nes-desktop/src/config.rs', 'r') as f:
        config_content = f.read()

    config_content = config_content.replace("}\n", "".join(config_tests) + "}\n")

    # with open('crates/nes-desktop/src/config.rs', 'w') as f:
    #     f.write(config_content)

    # with open('crates/nes-desktop/src/main.rs', 'w') as f:
    #     f.write(main_content)
