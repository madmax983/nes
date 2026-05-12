import re

with open('crates/nes-tui/src/main.rs', 'r') as f:
    text = f.read()

# Replace the function arguments and return type block, and the body.

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
            {arg7.strip()}
        )"""

text = re.sub(r'should_replace_protocol_state\(\n\s*(true|false),\n\s*(true|false),\n\s*(true|false),\n\s*(true|false),\n\s*([^,]+),\n\s*([^,]+),\n\s*([^)]+)\n\s*\)', replace_test_call, text)

# add import for ProtocolStateFlags in test module
text = text.replace("""use crate::{
        VideoBackend, VideoBackendKind, drain_protocol_results, draw_frame, evaluate_frame_tick,
        event_loop, fit_nes_viewport, format_rom_read_error, frame_rgba_to_rgba_image,
        handle_runtime_key_event, key_is_pressed, key_pressed_state, make_protocol_state,
        maybe_step_runtime_frame, parse_tui_args, protocol_image_resize, refresh_runtime_fps,
        select_video_backend_kind, should_quit, should_refresh_protocol_frame,
        should_replace_protocol_state, usage_line, usage_message,
    };""", """use crate::{
        ProtocolStateFlags, VideoBackend, VideoBackendKind, drain_protocol_results, draw_frame, evaluate_frame_tick,
        event_loop, fit_nes_viewport, format_rom_read_error, frame_rgba_to_rgba_image,
        handle_runtime_key_event, key_is_pressed, key_pressed_state, make_protocol_state,
        maybe_step_runtime_frame, parse_tui_args, protocol_image_resize, refresh_runtime_fps,
        select_video_backend_kind, should_quit, should_refresh_protocol_frame,
        should_replace_protocol_state, usage_line, usage_message,
    };""")

with open('crates/nes-tui/src/main.rs', 'w') as f:
    f.write(text)
