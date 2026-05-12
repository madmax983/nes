with open('crates/nes-tui/src/main.rs', 'r') as f:
    lines = f.readlines()

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

for i in range(len(lines)):
    if 'fn should_replace_protocol_state(' in lines[i]:
        # we will replace it!
        break

# wait, I will just write a simpler script using read and regex
