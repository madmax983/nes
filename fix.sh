cat << 'INNER_EOF' > main_patch.diff
<<<<<<< SEARCH
            if let Some(audio_output) = audio_output.as_ref() {
                if audio_output.queue_len() >= MAX_AUDIO_QUEUE_CHUNKS {
                    // Fast path: drain samples into a stack array without heap allocation
                    let mut dummy = [0_i16; nes_core::AUDIO_CHUNK_SAMPLES];
                    core.fill_audio_chunk_i16(&mut dummy);
                    metrics.on_audio_queue(audio_output.queue_len(), true);
                } else {
                    let queued = audio_output.queue_samples(core.audio_chunk_i16());
                    metrics.on_audio_queue(audio_output.queue_len(), audio_queue_dropped(queued));
                }
            }
=======
            if let Some(audio_output) = audio_output.as_ref() {
                if audio_output.queue_len() >= MAX_AUDIO_QUEUE_CHUNKS {
                    let mut dummy = [0_i16; nes_core::AUDIO_CHUNK_SAMPLES];
                    core.fill_audio_chunk_i16(&mut dummy);
                    metrics.on_audio_queue(audio_output.queue_len(), true);
                } else {
                    let mut audio_buf = [0_i16; nes_core::AUDIO_CHUNK_SAMPLES];
                    core.fill_audio_chunk_i16(&mut audio_buf);
                    let queued = audio_output.queue_samples(&audio_buf);
                    metrics.on_audio_queue(audio_output.queue_len(), audio_queue_dropped(queued));
                }
            }
>>>>>>> REPLACE
INNER_EOF
sed -i 's/fn append_i16(&self, samples: Vec<i16>);/fn append_i16(\&self, samples: \&[i16]);/g' crates/nes-desktop/src/audio.rs
sed -i 's/fn append_i16(&self, samples: Vec<i16>) {/fn append_i16(\&self, samples: \&[i16]) {/g' crates/nes-desktop/src/audio.rs
sed -i 's/pub fn queue_samples(&self, samples: Vec<i16>) -> bool {/pub fn queue_samples(\&self, samples: \&[i16]) -> bool {/g' crates/nes-desktop/src/audio.rs
sed -i 's/fn append_i16(&self, _samples: Vec<i16>) {}/fn append_i16(\&self, _samples: \&[i16]) {}/g' crates/nes-desktop/src/audio.rs
sed -i 's/output.queue_samples(vec!\[1_i16, 2, 3\])/output.queue_samples(\&[1_i16, 2, 3])/g' crates/nes-desktop/src/audio.rs
sed -i 's/adapter.append_i16(vec!\[/adapter.append_i16(\&\[/g' crates/nes-desktop/src/audio.rs
sed -i 's/1, 2, 3, 4\]);/1, 2, 3, 4\]);/g' crates/nes-desktop/src/audio.rs
sed -i 's/5, 6, 7, 8\]);/5, 6, 7, 8\]);/g' crates/nes-desktop/src/audio.rs
sed -i 's/adapter.append_i16(vec!\[i16::MAX; 5_000\])/adapter.append_i16(\&\[i16::MAX; 5_000\])/g' crates/nes-desktop/src/audio.rs
sed -i 's/audio.queue_samples(samples)/audio.queue_samples(\&samples)/g' crates/nes-desktop/src/audio.rs
