with open('crates/nes-tui/src/main.rs', 'r') as f:
    text = f.read()

import re

struct_def = """struct ProtocolStateFlags {
    has_protocol_state: bool,
    pending_resize: bool,
    area_needs_resize: bool,
    paused: bool,
}

"""

replacement = """fn should_replace_protocol_state(
    flags: ProtocolStateFlags,
    last_frame_update: Option<Instant>,
    now: Instant,
    interval: Duration,
) -> bool {
    if flags.pending_resize {
        return false;
    }
    if !flags.has_protocol_state {
        return true;
    }
    if flags.area_needs_resize {
        return true;
    }
    !flags.paused && should_refresh_protocol_frame(last_frame_update, now, interval)
}"""

# Do the replacements
text = re.sub(r'fn should_replace_protocol_state\([^{]*\) -> bool \{[^{]*if pending_resize \{[^{]*return false;\n    \}\n    if !has_protocol_state \{[^{]*return true;\n    \}\n    if area_needs_resize \{[^{]*return true;\n    \}\n    !paused && should_refresh_protocol_frame\(last_frame_update, now, interval\)\n\}', struct_def + replacement, text)

# update the call site in draw_frame
text = re.sub(r'let should_refresh = should_replace_protocol_state\(\n                has_protocol_state,\n                renderer\.pending_resize,\n                area_needs_resize\.is_some\(\),\n                runtime\.paused,\n                \*last_frame_update,\n                now,\n                render_interval,\n            \);', r'''let should_refresh = should_replace_protocol_state(
                ProtocolStateFlags {
                    has_protocol_state,
                    pending_resize: renderer.pending_resize,
                    area_needs_resize: area_needs_resize.is_some(),
                    paused: runtime.paused,
                },
                *last_frame_update,
                now,
                render_interval,
            );''', text)

# Now, we need to replace all test calls.
# I will use a regex to match all test calls to should_replace_protocol_state
# They look like: should_replace_protocol_state( b1, b2, b3, b4, last_frame, now, interval )

# 1: bool
# 2: bool
# 3: bool
# 4: bool
# 5: last_frame (up to next comma)
# 6: now (up to next comma)
# 7: interval (up to next parenthesis)

# Oh wait, some args like interval span lines. Let's just use Python's re.sub with a custom function.
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
            {arg7.strip()},
        )"""

text = re.sub(r'should_replace_protocol_state\(\n\s*(true|false),\n\s*(true|false),\n\s*(true|false),\n\s*(true|false),\n\s*([^,]+),\n\s*([^,]+),\n\s*([^,]+),\n\s*\)', replace_test_call, text)

with open('crates/nes-tui/src/main.rs', 'w') as f:
    f.write(text)
