use nes_mcp::publish_audio_with;

#[test]
#[should_panic(expected = "capacity overflow")]
#[ignore = "havoc target"]
fn havoc_test_publish_audio_with_capacity_overflow() {
    publish_audio_with(usize::MAX, |_| {});
}
