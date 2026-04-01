#!/bin/bash
git restore crates/nes-desktop/src/main.rs crates/nes-desktop/src/metrics.rs crates/nes-desktop/src/audio.rs crates/nes-desktop/src/lib.rs

cat << 'PY' > fix_test35.py
import re

with open("crates/nes-desktop/src/main.rs", "r") as f:
    content = f.read()

content = content.replace("use crate::audio::{AudioOutput, AudioSinkControl};", "use crate::audio::{AudioOutput, AudioSinkControl};\nuse crate::audio::MAX_AUDIO_QUEUE_CHUNKS;\nuse crate::audio::AUDIO_CHANNELS;")
content = content.replace("use crate::metrics::{PerfMetrics, MetricsSnapshot};", "use crate::metrics::{PerfMetrics, MetricsSnapshot};\nuse crate::metrics::DEFAULT_METRICS_EVERY_FRAMES;")
content = content.replace("audio_output.queue_len() >= MAX_AUDIO_QUEUE_CHUNKS", "audio_output.queue_len() >= crate::audio::MAX_AUDIO_QUEUE_CHUNKS")

content = content.replace("const DEFAULT_METRICS_EVERY_FRAMES: u64 = 60;", "")
content = content.replace("const MAX_AUDIO_QUEUE_CHUNKS: usize = 8;", "")
content = content.replace("const AUDIO_CHANNELS: u16 = 1;", "")

with open("crates/nes-desktop/src/main.rs", "w") as f:
    f.write(content)

with open("crates/nes-desktop/src/metrics.rs", "r") as f:
    metrics_content = f.read()

metrics_content = metrics_content.replace("use crate::DEFAULT_METRICS_EVERY_FRAMES;", "pub const DEFAULT_METRICS_EVERY_FRAMES: u64 = 60;\nuse crate::frame_signature;\nuse nes_core::NesCore;\nuse nes_config::normalize_nonzero_u64;\nuse crate::netplay::NetplayRuntimeStats;")
metrics_content = metrics_content.replace("crate::DEFAULT_METRICS_EVERY_FRAMES", "DEFAULT_METRICS_EVERY_FRAMES")

metrics_content = metrics_content.replace("pub(crate) struct", "pub struct")
metrics_content = metrics_content.replace("pub(crate) fn", "pub fn")
metrics_content = metrics_content.replace("pub(crate) trait", "pub trait")
metrics_content = metrics_content.replace("use super::*;", "use super::*;")


with open("crates/nes-desktop/src/metrics.rs", "w") as f:
    f.write(metrics_content)

with open("crates/nes-desktop/src/audio.rs", "r") as f:
    audio_content = f.read()

audio_content = audio_content.replace("use crate::{AUDIO_CHANNELS, MAX_AUDIO_QUEUE_CHUNKS};", "pub const MAX_AUDIO_QUEUE_CHUNKS: usize = 8;\npub const AUDIO_CHANNELS: u16 = 1;\nuse nes_core::AUDIO_SAMPLE_RATE;\nuse rodio::{buffer::SamplesBuffer, OutputStream, Sink};")
audio_content = audio_content.replace("crate::MAX_AUDIO_QUEUE_CHUNKS", "MAX_AUDIO_QUEUE_CHUNKS")

audio_content = audio_content.replace("pub(crate) struct", "pub struct")
audio_content = audio_content.replace("pub(crate) fn", "pub fn")
audio_content = audio_content.replace("pub(crate) trait", "pub trait")
audio_content = audio_content.replace("use super::*;", "use super::*;")


with open("crates/nes-desktop/src/audio.rs", "w") as f:
    f.write(audio_content)

with open("crates/nes-desktop/src/lib.rs", "r") as f:
    lib_content = f.read()
lib_content = lib_content.replace("pub mod app;\nmod audio;\nmod metrics;", "pub mod app;\npub mod audio;\npub mod metrics;")
with open("crates/nes-desktop/src/lib.rs", "w") as f:
    f.write(lib_content)
PY

python3 fix_test35.py
cargo test -p nes-desktop
