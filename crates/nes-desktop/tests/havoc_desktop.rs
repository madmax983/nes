use proptest::prelude::*;

fn recommended_input_delay_frames(
    rtt_ms: Option<f64>,
    jitter_ms: f64,
    min_delay_frames: u32,
    max_delay_frames: u32,
    current_delay_frames: u32,
) -> u32 {
    if min_delay_frames >= max_delay_frames {
        return min_delay_frames;
    }
    let Some(rtt_ms) = rtt_ms else {
        return current_delay_frames;
    };

    let frame_time_ms = 1_000.0 / 60.0;
    let estimated_one_way_ms = (rtt_ms * 0.5) + (jitter_ms * 1.5);
    let raw_target = (estimated_one_way_ms / frame_time_ms).ceil() as u32 + 1;
    let target = raw_target.clamp(min_delay_frames, max_delay_frames);

    if target > current_delay_frames {
        target.max(current_delay_frames.saturating_add(1))
    } else if target + 1 < current_delay_frames {
        current_delay_frames - 1
    } else {
        current_delay_frames
    }
}

proptest! {
    #[test]
    #[should_panic]
    fn test_recommended_input_delay_frames_panics_on_extreme_latency(
        _x in 0..1
    ) {
        let _ = recommended_input_delay_frames(Some(f64::MAX), 0.0, 0, 10, 5);
    }
}
