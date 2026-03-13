use nes_ai::obs::{FrameStack, downsample_grayscale};
use nes_core::{FRAME_HEIGHT, FRAME_WIDTH};

#[test]
fn grayscale_downsample_outputs_expected_shape_and_unit_range() {
    let mut rgba = vec![0_u8; FRAME_WIDTH * FRAME_HEIGHT * 4];
    rgba[0] = 255;
    rgba[1] = 255;
    rgba[2] = 255;
    rgba[3] = 255;

    let image = downsample_grayscale(&rgba, 84, 84);
    assert_eq!(image.len(), 84 * 84);
    assert!(image.iter().all(|value| (0.0..=1.0).contains(value)));
}

#[test]
fn frame_stack_retains_only_recent_frames() {
    let mut stack = FrameStack::new(2, 4);
    stack.push(vec![0.0; 4]);
    stack.push(vec![1.0; 4]);
    stack.push(vec![2.0; 4]);

    let frames = stack.as_slices();
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[0], &[1.0, 1.0, 1.0, 1.0]);
    assert_eq!(frames[1], &[2.0, 2.0, 2.0, 2.0]);
}
