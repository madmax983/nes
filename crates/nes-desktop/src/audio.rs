use nes_core::AUDIO_SAMPLE_RATE;
use rodio::buffer::SamplesBuffer;
use rodio::{OutputStream, Sink};

pub const MAX_AUDIO_QUEUE_CHUNKS: usize = 8;
pub const AUDIO_CHANNELS: u16 = 1;

pub trait AudioSinkControl {
    fn queue_len(&self) -> usize;
    fn append_i16(&self, samples: Vec<i16>);
    fn clear(&self);
    fn stop(&self);
}

pub struct RodioSinkAdapter {
    inner: Sink,
}

impl AudioSinkControl for RodioSinkAdapter {
    fn queue_len(&self) -> usize {
        self.inner.len()
    }

    fn append_i16(&self, samples: Vec<i16>) {
        self.inner.append(SamplesBuffer::new(
            AUDIO_CHANNELS,
            AUDIO_SAMPLE_RATE,
            samples,
        ));
    }

    fn clear(&self) {
        self.inner.clear();
    }

    fn stop(&self) {
        self.inner.stop();
    }
}

pub struct AudioOutput {
    sink: Box<dyn AudioSinkControl>,
    _stream: Option<OutputStream>,
}

impl AudioOutput {
    pub fn new_with_sink(sink: Box<dyn AudioSinkControl>, stream: Option<OutputStream>) -> Self {
        Self {
            sink,
            _stream: stream,
        }
    }

    pub fn try_new() -> Result<Self, String> {
        let (stream, handle) = OutputStream::try_default()
            .map_err(|err| format!("Audio output init failed: {err}"))?;
        let sink =
            Sink::try_new(&handle).map_err(|err| format!("Audio sink init failed: {err}"))?;
        let sink_adapter = RodioSinkAdapter { inner: sink };
        Ok(Self::new_with_sink(Box::new(sink_adapter), Some(stream)))
    }

    pub fn queue_samples(&self, samples: Vec<i16>) -> bool {
        if self.sink.queue_len() >= MAX_AUDIO_QUEUE_CHUNKS {
            return false;
        }
        self.sink.append_i16(samples);
        true
    }

    pub fn queue_len(&self) -> usize {
        self.sink.queue_len()
    }

    pub fn clear(&self) {
        self.sink.clear();
    }
}

impl Drop for AudioOutput {
    fn drop(&mut self) {
        // Ensure sink teardown happens while the stream backend is still alive.
        self.sink.stop();
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    pub struct FakeAudioState {
        pub queue_len: usize,
        pub append_calls: usize,
        pub appended_sample_count: usize,
        pub clear_calls: usize,
        pub stop_calls: usize,
    }

    pub struct FakeAudioSink {
        pub state: Arc<Mutex<FakeAudioState>>,
    }

    impl FakeAudioSink {
        pub fn with_len(initial_len: usize) -> (Self, Arc<Mutex<FakeAudioState>>) {
            let state = Arc::new(Mutex::new(FakeAudioState {
                queue_len: initial_len,
                ..Default::default()
            }));
            (
                Self {
                    state: Arc::clone(&state),
                },
                state,
            )
        }
    }

    impl AudioSinkControl for FakeAudioSink {
        fn queue_len(&self) -> usize {
            self.state
                .lock()
                .expect("fake audio state lock should not poison")
                .queue_len
        }

        fn append_i16(&self, samples: Vec<i16>) {
            let mut state = self
                .state
                .lock()
                .expect("fake audio state lock should not poison");
            state.append_calls = state.append_calls.saturating_add(1);
            state.appended_sample_count = state.appended_sample_count.saturating_add(samples.len());
            state.queue_len = state.queue_len.saturating_add(1);
        }

        fn stop(&self) {
            let mut state = self
                .state
                .lock()
                .expect("fake audio state lock should not poison");
            state.stop_calls = state.stop_calls.saturating_add(1);
        }

        fn clear(&self) {
            let mut state = self
                .state
                .lock()
                .expect("fake audio state lock should not poison");
            state.clear_calls = state.clear_calls.saturating_add(1);
            state.queue_len = 0;
        }
    }

    #[test]
    fn audio_output_queue_and_drop_behave_with_fake_sink() {
        let (fake_sink, shared_state) = FakeAudioSink::with_len(MAX_AUDIO_QUEUE_CHUNKS - 1);
        let output = AudioOutput::new_with_sink(Box::new(fake_sink), None);

        assert!(output.queue_samples(vec![1_i16, 2, 3]));
        assert_eq!(output.queue_len(), MAX_AUDIO_QUEUE_CHUNKS);

        let state_after_queue = shared_state
            .lock()
            .expect("fake audio state lock should not poison");
        assert_eq!(state_after_queue.append_calls, 1);
        assert_eq!(state_after_queue.appended_sample_count, 3);
        drop(state_after_queue);

        drop(output);
        let state_after_drop = shared_state
            .lock()
            .expect("fake audio state lock should not poison");
        assert_eq!(state_after_drop.stop_calls, 1);
    }

    #[test]
    fn audio_output_rejects_samples_when_queue_is_full() {
        let (fake_sink, shared_state) = FakeAudioSink::with_len(MAX_AUDIO_QUEUE_CHUNKS);
        let output = AudioOutput::new_with_sink(Box::new(fake_sink), None);

        assert!(!output.queue_samples(vec![1_i16, 2, 3]));
        assert_eq!(output.queue_len(), MAX_AUDIO_QUEUE_CHUNKS);

        let state = shared_state
            .lock()
            .expect("fake audio state lock should not poison");
        assert_eq!(state.append_calls, 0);
    }

    #[test]
    fn audio_output_clear_forwards_to_sink_and_empties_queue() {
        let (fake_sink, shared_state) = FakeAudioSink::with_len(3);
        let output = AudioOutput::new_with_sink(Box::new(fake_sink), None);

        output.clear();

        let state = shared_state
            .lock()
            .expect("fake audio state lock should not poison");
        assert_eq!(state.clear_calls, 1);
        assert_eq!(state.queue_len, 0);
    }

    #[test]
    fn rodio_sink_adapter_forwards_append_and_queue_len() {
        let (sink, _queue_rx) = Sink::new_idle();
        let adapter = RodioSinkAdapter { inner: sink };

        assert_eq!(adapter.queue_len(), 0);
        adapter.append_i16(vec![1_i16, 2, 3, 4]);
        assert_eq!(adapter.queue_len(), 1);
        adapter.append_i16(vec![5_i16, 6, 7, 8]);
        assert_eq!(adapter.queue_len(), 2);
    }

    #[test]
    fn rodio_sink_adapter_stop_mutes_idle_queue_output() {
        let (sink, mut queue_rx) = Sink::new_idle();
        let adapter = RodioSinkAdapter { inner: sink };

        adapter.append_i16(vec![i16::MAX; 5_000]);
        assert_eq!(
            queue_rx.next(),
            Some(0.999_969_5),
            "i16::MAX should map to a non-zero sample before stop"
        );

        adapter.stop();

        let saw_silence = (0..600).any(|_| queue_rx.next() == Some(0.0));
        assert!(saw_silence, "stop should silence the queued source");
    }
}
