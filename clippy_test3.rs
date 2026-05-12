pub struct ProtocolStateFlags {
    pub has_protocol_state: bool,
    pub pending_resize: bool,
    pub area_needs_resize: bool,
    pub paused: bool,
}

pub fn should_replace_protocol_state(
    flags: ProtocolStateFlags,
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
    !flags.paused
}
