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

# 2. Replace call site 1 (in draw_frame)
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

# 3. Replace all test calls using python string replace on exactly known test code from grep output!
text = text.replace("""        assert!(!should_replace_protocol_state(
            true,
            true,
            false,
            false,
            Some(Instant::now() - Duration::from_millis(34)),
            Instant::now(),
            Duration::from_millis(33)
        ));""", """        assert!(!should_replace_protocol_state(
            ProtocolStateFlags {
                has_protocol_state: true,
                pending_resize: true,
                area_needs_resize: false,
                paused: false,
            },
            Some(Instant::now() - Duration::from_millis(34)),
            Instant::now(),
            Duration::from_millis(33)
        ));""")

text = text.replace("""        assert!(should_replace_protocol_state(
            false,
            false,
            false,
            true,
            None,
            Instant::now(),
            Duration::from_millis(33)
        ));""", """        assert!(should_replace_protocol_state(
            ProtocolStateFlags {
                has_protocol_state: false,
                pending_resize: false,
                area_needs_resize: false,
                paused: true,
            },
            None,
            Instant::now(),
            Duration::from_millis(33)
        ));""")

text = text.replace("""        assert!(!should_replace_protocol_state(
            true,
            false,
            false,
            true,
            Some(now - interval),
            now,
            interval
        ));""", """        assert!(!should_replace_protocol_state(
            ProtocolStateFlags {
                has_protocol_state: true,
                pending_resize: false,
                area_needs_resize: false,
                paused: true,
            },
            Some(now - interval),
            now,
            interval
        ));""")

text = text.replace("""        assert!(!should_replace_protocol_state(
            true,
            false,
            false,
            false,
            Some(now - Duration::from_millis(10)),
            now,
            interval
        ));""", """        assert!(!should_replace_protocol_state(
            ProtocolStateFlags {
                has_protocol_state: true,
                pending_resize: false,
                area_needs_resize: false,
                paused: false,
            },
            Some(now - Duration::from_millis(10)),
            now,
            interval
        ));""")

text = text.replace("""        assert!(should_replace_protocol_state(
            true,
            false,
            true,
            true,
            Some(Instant::now()),
            Instant::now(),
            Duration::from_millis(33)
        ));""", """        assert!(should_replace_protocol_state(
            ProtocolStateFlags {
                has_protocol_state: true,
                pending_resize: false,
                area_needs_resize: true,
                paused: true,
            },
            Some(Instant::now()),
            Instant::now(),
            Duration::from_millis(33)
        ));""")


text = text.replace("""        should_replace_protocol_state, usage_line, usage_message,
    };""", """        ProtocolStateFlags, should_replace_protocol_state, usage_line, usage_message,
    };""")

with open('crates/nes-tui/src/main.rs', 'w') as f:
    f.write(text)
