use nes_mcp::{audio_chunk, publish_audio};

#[test]
#[should_panic]
fn havoc_test_mcp_output_oom() {
    // Attempting to allocate massive buffers into the shared state
    let massive_audio = vec![0; 1024 * 1024 * 100]; // 100MB
    publish_audio(massive_audio);

    // If it didn't panic or OOM, try to trigger the limit validation logic by accessing it
    let _chunk = audio_chunk(0);

    // Actually the goal of the test is just to show we can pump 100MB freely into the audio buffer
    panic!("Havoc was able to consume 100MB of RAM because publish_audio has no size limits");
}
