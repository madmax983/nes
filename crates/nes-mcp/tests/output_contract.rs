use nes_core::AUDIO_CHUNK_SAMPLES;
use nes_mcp::{audio_chunk, frame_chunk, latest_output_metadata};

#[test]
fn metadata_reports_incrementing_frame_sequence() {
    let m0 = latest_output_metadata();
    let frame = frame_chunk(m0.frame_seq + 1).expect("frame chunk available");
    let m1 = latest_output_metadata();
    assert!(m1.frame_seq >= m0.frame_seq);
    assert_eq!(
        frame.rgba.len(),
        usize::try_from(m1.width).unwrap() * usize::try_from(m1.height).unwrap() * 4
    );
}

#[test]
fn metadata_reports_incrementing_audio_sequence() {
    let m0 = latest_output_metadata();
    let chunk = audio_chunk(m0.audio_seq + 1).expect("audio chunk available");
    let m1 = latest_output_metadata();
    assert!(m1.audio_seq >= m0.audio_seq);
    assert_eq!(chunk.samples.len(), AUDIO_CHUNK_SAMPLES);
}
