import re

with open('crates/nes-tui/src/main.rs', 'r') as f:
    text = f.read()

# I am replacing the function arguments and return type block, and the body.
# Then I need to replace the call sites. Let's just use Python string replacement where regex fails.

text = text.replace("""fn should_replace_protocol_state(
    has_protocol_state: bool,
    pending_resize: bool,
    area_needs_resize: bool,
    paused: bool,
    last_frame_update: Option<Instant>,
    now: Instant,
    interval: Duration,
) -> bool {
    if pending_resize {
        return false;
    }
    if !has_protocol_state {
        return true;
    }
    if area_needs_resize {
        return true;
    }
    !paused && should_refresh_protocol_frame(last_frame_update, now, interval)
}""", """struct ProtocolStateFlags {
    has_protocol_state: bool,
    pending_resize: bool,
    area_needs_resize: bool,
    paused: bool,
}

fn should_replace_protocol_state(
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
}""")

text = text.replace("""let should_refresh = should_replace_protocol_state(
                has_protocol_state,
                renderer.pending_resize,
                area_needs_resize.is_some(),
                runtime.paused,
                *last_frame_update,
                now,
                render_interval,
            );""", """let should_refresh = should_replace_protocol_state(
                ProtocolStateFlags {
                    has_protocol_state,
                    pending_resize: renderer.pending_resize,
                    area_needs_resize: area_needs_resize.is_some(),
                    paused: runtime.paused,
                },
                *last_frame_update,
                now,
                render_interval,
            );""")

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

# one more test call:
text = re.sub(r'should_replace_protocol_state\(\n\s*false,\n\s*false,\n\s*false,\n\s*true,\n\s*None,\n\s*Instant::now\(\),\n\s*Duration::from_millis\(33\),\n\s*\)', replace_test_call, text)


# Wait, wait, let me just see the original file using grep again to get ALL exact test calls.
