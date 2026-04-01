import re

with open("crates/nes-desktop/src/main.rs", "r") as f:
    content = f.read()

# We'll just define the constants in main.rs and also expose them via pub
content = content.replace("const DEFAULT_METRICS_EVERY_FRAMES: u64 = 60;", "pub(crate) const DEFAULT_METRICS_EVERY_FRAMES: u64 = 60;")
content = content.replace("const MAX_AUDIO_QUEUE_CHUNKS: usize = 8;", "pub(crate) const MAX_AUDIO_QUEUE_CHUNKS: usize = 8;")
content = content.replace("const AUDIO_CHANNELS: u16 = 1;", "pub(crate) const AUDIO_CHANNELS: u16 = 1;")

content = content.replace("use rodio::{OutputStream, Sink, buffer::SamplesBuffer};", "use crate::audio::{AudioOutput, AudioSinkControl};\nuse crate::metrics::{PerfMetrics, MetricsSnapshot};\nuse rodio::{OutputStream, Sink, buffer::SamplesBuffer};")


# Now, strip audio code from main
content = re.sub(r"trait AudioSinkControl \{.*?\n\}\n+", "", content, flags=re.DOTALL)
content = re.sub(r"struct RodioSinkAdapter \{.*?\n\}\n+", "", content, flags=re.DOTALL)
content = re.sub(r"impl AudioSinkControl for RodioSinkAdapter \{.*?\n\}\n+", "", content, flags=re.DOTALL)
content = re.sub(r"struct AudioOutput \{.*?\n\}\n+", "", content, flags=re.DOTALL)
content = re.sub(r"impl AudioOutput \{.*?\n\}\n+", "", content, flags=re.DOTALL)
content = re.sub(r"impl Drop for AudioOutput \{.*?\n\}\n+", "", content, flags=re.DOTALL)

# Strip metric code from main
content = re.sub(r"struct PerfMetrics \{.*?\n\}\n+", "", content, flags=re.DOTALL)
content = re.sub(r"#\[derive\(Debug, Clone, Copy, PartialEq\)\]\nstruct MetricsSnapshot \{.*?\n\}\n+", "", content, flags=re.DOTALL)
content = re.sub(r"fn compute_metrics_snapshot\(.*?\n\}\n+", "", content, flags=re.DOTALL)
content = re.sub(r"impl PerfMetrics \{.*?\n\}\n+", "", content, flags=re.DOTALL)

# Strip test
content = re.sub(r"    #\[test\]\n    fn compute_metrics_snapshot_derives_expected_rates\(\) \{.*?\n    \}\n+", "", content, flags=re.DOTALL)
content = re.sub(r"    #\[test\]\n    fn compute_metrics_snapshot_handles_guard_conditions_and_saturating_ppu_delta\(\) \{.*?\n    \}\n+", "", content, flags=re.DOTALL)
content = re.sub(r"    #\[test\]\n    fn perf_metrics_on_step_tracks_stalls_and_recovers_on_pc_change\(\) \{.*?\n    \}\n+", "", content, flags=re.DOTALL)
content = re.sub(r"    #\[test\]\n    fn perf_metrics_render_audio_and_netplay_observation_update_fields\(\) \{.*?\n    \}\n+", "", content, flags=re.DOTALL)
content = re.sub(r"    #\[test\]\n    fn perf_metrics_maybe_report_resets_window_after_threshold\(\) \{.*?\n    \}\n+", "", content, flags=re.DOTALL)
content = re.sub(r"    #\[test\]\n    fn perf_metrics_maybe_report_guard_paths_skip_when_disabled_or_under_threshold\(\) \{.*?\n    \}\n+", "", content, flags=re.DOTALL)
content = re.sub(r"    #\[test\]\n    fn perf_metrics_disabled_mode_does_not_mutate_tracking_fields\(\) \{.*?\n    \}\n+", "", content, flags=re.DOTALL)

content = re.sub(r"    #\[test\]\n    fn audio_output_queue_and_drop_behave_with_fake_sink\(\) \{.*?\n    \}\n+", "", content, flags=re.DOTALL)
content = re.sub(r"    #\[test\]\n    fn audio_output_rejects_samples_when_queue_is_full\(\) \{.*?\n    \}\n+", "", content, flags=re.DOTALL)
content = re.sub(r"    #\[test\]\n    fn audio_output_clear_forwards_to_sink_and_empties_queue\(\) \{.*?\n    \}\n+", "", content, flags=re.DOTALL)
content = re.sub(r"    #\[test\]\n    fn rodio_sink_adapter_stop_mutes_idle_queue_output\(\) \{.*?\n    \}\n+", "", content, flags=re.DOTALL)
content = re.sub(r"    #\[test\]\n    fn rodio_sink_adapter_forwards_append_and_queue_len\(\) \{.*?\n    \}\n+", "", content, flags=re.DOTALL)


content = re.sub(r"    struct FakeAudioState \{.*?\n    \}\n+", "", content, flags=re.DOTALL)
content = re.sub(r"    struct FakeAudioSink \{.*?\n    \}\n+", "", content, flags=re.DOTALL)
content = re.sub(r"    impl AudioSinkControl for FakeAudioSink \{.*?\n    \}\n+", "", content, flags=re.DOTALL)
content = re.sub(r"    #\[derive\(Debug, Default\)\]\n    impl FakeAudioSink \{.*?\n    \}\n\n", "", content, flags=re.DOTALL)

content = re.sub(r"        MetricsSnapshot, NetplayRuntimeStats, PerfMetrics, RodioSinkAdapter, RuntimeArgs, StepMode,\n", "        NetplayRuntimeStats, RuntimeArgs, StepMode,\n", content, flags=re.DOTALL)
content = re.sub(r"        classify_keyboard_input, classify_window_event, compute_metrics_snapshot,\n", "        classify_keyboard_input, classify_window_event,\n", content, flags=re.DOTALL)

with open("crates/nes-desktop/src/main.rs", "w") as f:
    f.write(content)

with open("crates/nes-desktop/src/metrics.rs", "r") as f:
    metrics_content = f.read()

metrics_content = metrics_content.replace("crate::DEFAULT_METRICS_EVERY_FRAMES", "crate::DEFAULT_METRICS_EVERY_FRAMES")
metrics_content = metrics_content.replace("use super::*;\n    use std::time::Instant;", "use super::*;\n    use std::time::Instant;\n    use crate::DEFAULT_METRICS_EVERY_FRAMES;")

with open("crates/nes-desktop/src/metrics.rs", "w") as f:
    f.write(metrics_content)

with open("crates/nes-desktop/src/audio.rs", "r") as f:
    audio_content = f.read()

audio_content = audio_content.replace("crate::MAX_AUDIO_QUEUE_CHUNKS", "crate::MAX_AUDIO_QUEUE_CHUNKS")

with open("crates/nes-desktop/src/audio.rs", "w") as f:
    f.write(audio_content)

with open("crates/nes-desktop/src/lib.rs", "r") as f:
    lib_content = f.read()
lib_content = lib_content.replace("pub mod app;", "pub mod app;\npub mod audio;\npub mod metrics;")
with open("crates/nes-desktop/src/lib.rs", "w") as f:
    f.write(lib_content)
