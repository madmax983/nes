import re

with open('crates/nes-desktop/src/main.rs', 'r') as f:
    main_content = f.read()

# We need to find the `mod tests` block and extract tests into config.rs
tests_block_re = re.compile(r'#\[cfg\(test\)\]\nmod tests \{.*?\n\}\n', re.DOTALL)
tests_match = tests_block_re.search(main_content)

if tests_match:
    tests_content = tests_match.group(0)

    config_tests = []

    config_funcs = ["adaptive_delay_reacts_to_rtt_and_jitter", "adaptive_delay_uses_current_when_no_rtt_sample", "adaptive_delay_exact_targets_and_hysteresis_behave_as_expected", "adaptive_delay_returns_min_when_bounds_are_invalid", "capture_config_helpers_handle_placeholders_and_defaults"]

    for match in re.finditer(r'(#\[test\]\s+fn\s+(\w+)\(.*?\)\s*\{.*?^\s{4}\})', tests_content, re.DOTALL | re.MULTILINE):
        func_body = match.group(1)
        func_name = match.group(2)
        if func_name in config_funcs:
            config_tests.append(func_body)
            # Remove from main_content
            main_content = main_content.replace(func_body, "")

    # Write to config.rs
    with open('crates/nes-desktop/src/config.rs', 'a') as f:
        f.write("\n#[cfg(test)]\nmod tests {\n    use super::*;\n\n")
        for test in config_tests:
            # Need to adjust indentation
            lines = test.split('\n')
            for line in lines:
                if line.startswith('    '):
                    f.write(line[4:] + "\n")
                else:
                    f.write(line + "\n")
        f.write("}\n")

    with open('crates/nes-desktop/src/main.rs', 'w') as f:
        f.write(main_content)
