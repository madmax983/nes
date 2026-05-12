import re

with open('crates/nes-tui/src/main.rs', 'r') as f:
    text = f.read()

# Replace test calls:
def replace_test_call(match):
    b1, b2, b3, b4, arg5, arg6, arg7 = match.groups()
    return f"""should_replace_protocol_state(
            ProtocolStateFlags {{
                has_protocol_state: {b1.strip()},
                pending_resize: {b2.strip()},
                area_needs_resize: {b3.strip()},
                paused: {b4.strip()},
            }},
            {arg5.strip()},
            {arg6.strip()},
            {arg7.strip()}
        )"""

text = re.sub(r'should_replace_protocol_state\(\n\s*(true|false),\n\s*(true|false),\n\s*(true|false),\n\s*(true|false),\n\s*([^,]+),\n\s*([^,]+),\n\s*([^)]+)\n\s*\)', replace_test_call, text)

with open('crates/nes-tui/src/main.rs', 'w') as f:
    f.write(text)
