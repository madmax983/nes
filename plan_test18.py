import re

with open('crates/nes-tui/src/main.rs', 'r') as f:
    text = f.read()

# Replace definition
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

text = re.sub(r'fn should_replace_protocol_state\(\s*has_protocol_state: bool,\s*pending_resize: bool,\s*area_needs_resize: bool,\s*paused: bool,\s*last_frame_update: Option<Instant>,\s*now: Instant,\s*interval: Duration,\s*\) -> bool \{\s*if pending_resize \{\s*return false;\s*\}\s*if !has_protocol_state \{\s*return true;\s*\}\s*if area_needs_resize \{\s*return true;\s*\}\s*!paused && should_refresh_protocol_frame\(last_frame_update, now, interval\)\s*\}', struct_def + replacement, text)


test1_old = """        assert!(!should_replace_protocol_state(
            true,
            true,
            false,
            false,
            Some(Instant::now() - Duration::from_millis(34)),
            Instant::now(),
            Duration::from_millis(33)
        ));"""
test1_new = """        assert!(!should_replace_protocol_state(
            ProtocolStateFlags {
                has_protocol_state: true,
                pending_resize: true,
                area_needs_resize: false,
                paused: false,
            },
            Some(Instant::now() - Duration::from_millis(34)),
            Instant::now(),
            Duration::from_millis(33)
        ));"""

test2_old = """        assert!(should_replace_protocol_state(
            false,
            false,
            false,
            true,
            None,
            Instant::now(),
            Duration::from_millis(33)
        ));"""
test2_new = """        assert!(should_replace_protocol_state(
            ProtocolStateFlags {
                has_protocol_state: false,
                pending_resize: false,
                area_needs_resize: false,
                paused: true,
            },
            None,
            Instant::now(),
            Duration::from_millis(33)
        ));"""

test3_old = """        assert!(!should_replace_protocol_state(
            true,
            false,
            false,
            true,
            Some(now - interval),
            now,
            interval
        ));"""
test3_new = """        assert!(!should_replace_protocol_state(
            ProtocolStateFlags {
                has_protocol_state: true,
                pending_resize: false,
                area_needs_resize: false,
                paused: true,
            },
            Some(now - interval),
            now,
            interval
        ));"""

test4_old = """        assert!(!should_replace_protocol_state(
            true,
            false,
            false,
            false,
            Some(now - Duration::from_millis(10)),
            now,
            interval
        ));"""
test4_new = """        assert!(!should_replace_protocol_state(
            ProtocolStateFlags {
                has_protocol_state: true,
                pending_resize: false,
                area_needs_resize: false,
                paused: false,
            },
            Some(now - Duration::from_millis(10)),
            now,
            interval
        ));"""

test5_old = """        assert!(should_replace_protocol_state(
            true,
            false,
            true,
            true,
            Some(Instant::now()),
            Instant::now(),
            Duration::from_millis(33)
        ));"""
test5_new = """        assert!(should_replace_protocol_state(
            ProtocolStateFlags {
                has_protocol_state: true,
                pending_resize: false,
                area_needs_resize: true,
                paused: true,
            },
            Some(Instant::now()),
            Instant::now(),
            Duration::from_millis(33)
        ));"""

test6_old = """        assert!(should_replace_protocol_state(
            true,
            false,
            false,
            false,
            Some(now - interval),
            now,
            interval
        ));"""
test6_new = """        assert!(should_replace_protocol_state(
            ProtocolStateFlags {
                has_protocol_state: true,
                pending_resize: false,
                area_needs_resize: false,
                paused: false,
            },
            Some(now - interval),
            now,
            interval
        ));"""

text = text.replace(test1_old, test1_new)
text = text.replace(test2_old, test2_new)
text = text.replace(test3_old, test3_new)
text = text.replace(test4_old, test4_new)
text = text.replace(test5_old, test5_new)
text = text.replace(test6_old, test6_new)

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

with open('crates/nes-tui/src/main.rs', 'w') as f:
    f.write(text)
