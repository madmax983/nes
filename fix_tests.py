import re

with open('crates/nes-desktop/src/main.rs', 'r') as f:
    main_content = f.read()

tests_block_re = re.compile(r'#\[cfg\(test\)\]\nmod tests \{.*?\n\}\n', re.DOTALL)
tests_match = tests_block_re.search(main_content)

if tests_match:
    tests_content = tests_match.group(0)

    session_tests = []

    session_funcs = ["format_rom_read_error_handles_not_found_and_other_errors"]

    for match in re.finditer(r'(#\[test\]\s+fn\s+(\w+)\(.*?\)\s*\{.*?^\s{4}\})', tests_content, re.DOTALL | re.MULTILINE):
        func_body = match.group(1)
        func_name = match.group(2)
        if func_name in session_funcs:
            session_tests.append(func_body)
            main_content = main_content.replace(func_body, "")

    with open('crates/nes-desktop/src/session.rs', 'a') as f:
        f.write("\n#[cfg(test)]\nmod tests {\n    use super::*;\n\n")
        for test in session_tests:
            lines = test.split('\n')
            for line in lines:
                if line.startswith('    '):
                    f.write(line[4:] + "\n")
                else:
                    f.write(line + "\n")
        f.write("}\n")

    with open('crates/nes-desktop/src/main.rs', 'w') as f:
        f.write(main_content)
