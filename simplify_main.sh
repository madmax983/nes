cat << 'INNER_EOF' > main_patch.diff
<<<<<<< SEARCH
            if let Some(audio_output) = audio_output.as_ref() {
                if audio_output.queue_len() >= MAX_AUDIO_QUEUE_CHUNKS {
                    // Fast path: drain samples into a stack array without heap allocation
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
=======
            if let Some(audio_output) = audio_output.as_ref() {
                let mut audio_buf = [0_i16; nes_core::AUDIO_CHUNK_SAMPLES];
                core.fill_audio_chunk_i16(&mut audio_buf);
                if audio_output.queue_len() >= MAX_AUDIO_QUEUE_CHUNKS {
                    metrics.on_audio_queue(audio_output.queue_len(), true);
                } else {
                    let queued = audio_output.queue_samples(&audio_buf);
                    metrics.on_audio_queue(audio_output.queue_len(), audio_queue_dropped(queued));
                }
            }
>>>>>>> REPLACE
INNER_EOF
