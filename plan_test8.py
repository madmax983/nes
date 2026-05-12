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

text = text.replace("""assert!(!should_replace_protocol_state(
            true,
            true,
            false,
            false,
            Some(Instant::now() - Duration::from_millis(34)),
            Instant::now(),
            Duration::from_millis(33),
        ));""", """assert!(!should_replace_protocol_state(
            ProtocolStateFlags {
                has_protocol_state: true,
                pending_resize: true,
                area_needs_resize: false,
                paused: false,
            },
            Some(Instant::now() - Duration::from_millis(34)),
            Instant::now(),
            Duration::from_millis(33),
        ));""")

text = text.replace("""assert!(should_replace_protocol_state(
            false,
            false,
            false,
            true,
            None,
            Instant::now(),
            Duration::from_millis(33),
        ));""", """assert!(should_replace_protocol_state(
            ProtocolStateFlags {
                has_protocol_state: false,
                pending_resize: false,
                area_needs_resize: false,
                paused: true,
            },
            None,
            Instant::now(),
            Duration::from_millis(33),
        ));""")

text = text.replace("""assert!(!should_replace_protocol_state(
            true,
            false,
            false,
            true,
            Some(now - interval),
            now,
            interval,
        ));""", """assert!(!should_replace_protocol_state(
            ProtocolStateFlags {
                has_protocol_state: true,
                pending_resize: false,
                area_needs_resize: false,
                paused: true,
            },
            Some(now - interval),
            now,
            interval,
        ));""")

text = text.replace("""assert!(!should_replace_protocol_state(
            true,
            false,
            false,
            false,
            Some(now - Duration::from_millis(10)),
            now,
            interval,
        ));""", """assert!(!should_replace_protocol_state(
            ProtocolStateFlags {
                has_protocol_state: true,
                pending_resize: false,
                area_needs_resize: false,
                paused: false,
            },
            Some(now - Duration::from_millis(10)),
            now,
            interval,
        ));""")

text = text.replace("""assert!(should_replace_protocol_state(
            true,
            false,
            true,
            true,
            Some(Instant::now()),
            Instant::now(),
            Duration::from_millis(33),
        ));""", """assert!(should_replace_protocol_state(
            ProtocolStateFlags {
                has_protocol_state: true,
                pending_resize: false,
                area_needs_resize: true,
                paused: true,
            },
            Some(Instant::now()),
            Instant::now(),
            Duration::from_millis(33),
        ));""")


with open('crates/nes-tui/src/main.rs', 'w') as f:
    f.write(text)
