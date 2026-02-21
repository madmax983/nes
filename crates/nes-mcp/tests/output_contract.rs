use nes_mcp::{audio_chunk, frame_chunk, latest_output_metadata};

#[test]
fn metadata_reports_incrementing_frame_sequence() {
    let m0 = latest_output_metadata();
    let _ = frame_chunk(m0.frame_seq + 1);
    let m1 = latest_output_metadata();
    assert!(m1.frame_seq >= m0.frame_seq);
}

#[test]
fn metadata_reports_incrementing_audio_sequence() {
    let m0 = latest_output_metadata();
    let _ = audio_chunk(m0.audio_seq + 1);
    let m1 = latest_output_metadata();
    assert!(m1.audio_seq >= m0.audio_seq);
}
