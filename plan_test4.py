with open('crates/nes-tui/src/main.rs', 'r') as f:
    text = f.read()

import re

struct_def = """
struct ProtocolStateFlags {
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
text = re.sub(r'fn should_replace_protocol_state\([^{]*\) -> bool \{[^{]*if pending_resize \{[^{]*return false;\n    \}\n    if !has_protocol_state \{[^{]*return true;\n    \}\n    if area_needs_resize \{[^{]*return true;\n    \}\n    !paused && should_refresh_protocol_frame\(last_frame_update, now, interval\)\n\}', replacement, text)

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


text = re.sub(r'assert\!\(\!should_replace_protocol_state\(\n            true,\n            true,\n            false,\n            false,\n            Some\(Instant::now\(\) - Duration::from_millis\(34\)\),\n            Instant::now\(\),\n            Duration::from_millis\(33\),\n        \)\);', r'''assert!(!should_replace_protocol_state(
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

text = re.sub(r'assert\!\(should_replace_protocol_state\(\n            false,\n            false,\n            false,\n            true,\n            None,\n            Instant::now\(\),\n            Duration::from_millis\(33\),\n        \)\);', r'''assert!(should_replace_protocol_state(
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

text = re.sub(r'assert\!\(\!should_replace_protocol_state\(\n            true,\n            false,\n            false,\n            true,\n            Some\(now - interval\),\n            now,\n            interval,\n        \)\);', r'''assert!(!should_replace_protocol_state(
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

text = re.sub(r'assert\!\(\!should_replace_protocol_state\(\n            true,\n            false,\n            false,\n            false,\n            Some\(now - Duration::from_millis\(10\)\),\n            now,\n            interval,\n        \)\);', r'''assert!(!should_replace_protocol_state(
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

text = re.sub(r'assert\!\(should_replace_protocol_state\(\n            true,\n            false,\n            true,\n            true,\n            Some\(Instant::now\(\)\),\n            Instant::now\(\),\n            Duration::from_millis\(33\),\n        \)\);', r'''assert!(should_replace_protocol_state(
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

# add struct definition before should_replace_protocol_state
text = re.sub(r'fn should_replace_protocol_state', struct_def + 'fn should_replace_protocol_state', text)


with open('crates/nes-tui/src/main.rs', 'w') as f:
    f.write(text)
