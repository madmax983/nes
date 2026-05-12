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

# 1. Replace definition
text = re.sub(r'fn should_replace_protocol_state\(\s*has_protocol_state: bool,\s*pending_resize: bool,\s*area_needs_resize: bool,\s*paused: bool,\s*last_frame_update: Option<Instant>,\s*now: Instant,\s*interval: Duration,\s*\) -> bool \{\s*if pending_resize \{\s*return false;\s*\}\s*if !has_protocol_state \{\s*return true;\s*\}\s*if area_needs_resize \{\s*return true;\s*\}\s*!paused && should_refresh_protocol_frame\(last_frame_update, now, interval\)\s*\}', struct_def + replacement, text)

text = re.sub(r'let should_refresh = should_replace_protocol_state\(\s*has_protocol_state,\s*renderer\.pending_resize,\s*area_needs_resize\.is_some\(\),\s*runtime\.paused,\s*\*last_frame_update,\s*now,\s*render_interval,\s*\);', r'''let should_refresh = should_replace_protocol_state(
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

text = re.sub(r'assert!\(!should_replace_protocol_state\(\s*true,\s*true,\s*false,\s*false,\s*Some\(Instant::now\(\) - Duration::from_millis\(34\)\),\s*Instant::now\(\),\s*Duration::from_millis\(33\),\s*\)\);', r'''assert!(!should_replace_protocol_state(
            ProtocolStateFlags {
                has_protocol_state: true,
                pending_resize: true,
                area_needs_resize: false,
                paused: false,
            },
            Some(Instant::now() - Duration::from_millis(34)),
            Instant::now(),
            Duration::from_millis(33),
        ));''', text)

text = re.sub(r'assert!\(should_replace_protocol_state\(\s*false,\s*false,\s*false,\s*true,\s*None,\s*Instant::now\(\),\s*Duration::from_millis\(33\),\s*\)\);', r'''assert!(should_replace_protocol_state(
            ProtocolStateFlags {
                has_protocol_state: false,
                pending_resize: false,
                area_needs_resize: false,
                paused: true,
            },
            None,
            Instant::now(),
            Duration::from_millis(33),
        ));''', text)

text = re.sub(r'assert!\(!should_replace_protocol_state\(\s*true,\s*false,\s*false,\s*true,\s*Some\(now - interval\),\s*now,\s*interval,\s*\)\);', r'''assert!(!should_replace_protocol_state(
            ProtocolStateFlags {
                has_protocol_state: true,
                pending_resize: false,
                area_needs_resize: false,
                paused: true,
            },
            Some(now - interval),
            now,
            interval,
        ));''', text)

text = re.sub(r'assert!\(!should_replace_protocol_state\(\s*true,\s*false,\s*false,\s*false,\s*Some\(now - Duration::from_millis\(10\)\),\s*now,\s*interval,\s*\)\);', r'''assert!(!should_replace_protocol_state(
            ProtocolStateFlags {
                has_protocol_state: true,
                pending_resize: false,
                area_needs_resize: false,
                paused: false,
            },
            Some(now - Duration::from_millis(10)),
            now,
            interval,
        ));''', text)

text = re.sub(r'assert!\(should_replace_protocol_state\(\s*true,\s*false,\s*true,\s*true,\s*Some\(Instant::now\(\)\),\s*Instant::now\(\),\s*Duration::from_millis\(33\),\s*\)\);', r'''assert!(should_replace_protocol_state(
            ProtocolStateFlags {
                has_protocol_state: true,
                pending_resize: false,
                area_needs_resize: true,
                paused: true,
            },
            Some(Instant::now()),
            Instant::now(),
            Duration::from_millis(33),
        ));''', text)

# Just in case, add the struct import
text = re.sub(r'should_replace_protocol_state, usage_line, usage_message,', r'ProtocolStateFlags, should_replace_protocol_state, usage_line, usage_message,', text)


with open('crates/nes-tui/src/main.rs', 'w') as f:
    f.write(text)
