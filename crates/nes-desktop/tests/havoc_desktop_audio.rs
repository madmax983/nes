use nes_desktop::audio::AudioOutput;

#[test]
#[should_panic]
fn havoc_test_audio_output_dos() {
    let stream = AudioOutput::try_new().expect("AudioOutput should init");

    // We send a massive slice of garbage data to the audio queue to block / crash it.
    let massive_chunk = vec![i16::MAX; 1024 * 1024 * 100]; // 100 million shorts ~ 200MB
    stream.queue_samples(massive_chunk.clone());

    // Attempt to queue it over and over. If there's an unbounded queue, OOM or hang.
    for _ in 0..10 {
        stream.queue_samples(massive_chunk.clone());
    }

    panic!("Havoc was able to queue massive garbage float audio into the un-bounded audio stream.");
}
