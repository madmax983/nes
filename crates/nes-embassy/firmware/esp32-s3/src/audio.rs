//! I2S audio output: 44.1 kHz stereo 16-bit Philips format, fed by DMA.
//!
//! The core produces one mono [`AUDIO_CHUNK_SAMPLES`] chunk per frame; we
//! duplicate it to stereo in a DMA buffer and let the peripheral clock it
//! out while the next frame emulates.

use esp_hal::{
    dma::DmaTxBuf,
    i2s::master::{Channels, DataFormat, I2s, I2sTx, TdmConfig},
    peripherals::{DMA_CH0, I2S0},
    time::Rate,
    Async,
};
use nes_embassy::AUDIO_CHUNK_SAMPLES;

use crate::board::{self, steal_pin};

/// Mono samples per DMA transfer (one core audio chunk).
pub const CHUNK_SAMPLES: usize = AUDIO_CHUNK_SAMPLES;
/// Stereo frame bytes per transfer: 735 samples * 2 channels * 2 bytes.
pub const DMA_BYTES: usize = CHUNK_SAMPLES * 2 * 2;

/// I2S transmitter plus its configuration. The DMA buffer is passed
/// in/out of [`AudioOut::push_chunk`] because the transfer owns it while
/// it runs.
pub struct AudioOut {
    #[cfg_attr(feature = "qemu", allow(dead_code))]
    tx: Option<I2sTx<'static, Async>>,
}

/// Errors that can escape audio setup or playback.
#[derive(Debug)]
pub enum AudioError {
    Config,
    /// The transmitter was already checked out; cannot happen in the
    /// single-task main loop, but it is an error rather than a panic.
    TransmitterBusy,
}

pub fn init(i2s: I2S0<'static>, dma_channel: DMA_CH0<'static>) -> Result<AudioOut, AudioError> {
    let i2s = I2s::new(
        i2s,
        dma_channel,
        TdmConfig::new_tdm_philips()
            .with_sample_rate(Rate::from_hz(44_100))
            .with_data_format(DataFormat::Data16Channel16)
            .with_channels(Channels::STEREO),
    )
    .map_err(|_| AudioError::Config)?;

    let tx = i2s
        .into_async()
        .i2s_tx
        .with_bclk(steal_pin(board::I2S_BCLK))
        .with_ws(steal_pin(board::I2S_WS))
        .with_dout(steal_pin(board::I2S_DOUT))
        .build();

    Ok(AudioOut { tx: Some(tx) })
}

impl AudioOut {
    /// Pushes one mono chunk to the DAC, duplicated to stereo.
    ///
    /// Takes the DMA buffer, fills it, DMAs it out, waits for completion,
    /// and hands the buffer back. Blocking on completion keeps the audio
    /// clock honest: if emulation ever runs faster than real time, this is
    /// what holds it to the audio rate.
    ///
    /// With the `qemu` feature the DMA transfer itself is skipped (QEMU
    /// models no audio codec); the stereo duplication into the DMA buffer
    /// still runs.
    pub async fn push_chunk(
        &mut self,
        mono: &[i16],
        mut dma_buf: DmaTxBuf,
    ) -> Result<DmaTxBuf, AudioError> {
        debug_assert_eq!(mono.len(), CHUNK_SAMPLES);
        debug_assert!(dma_buf.len() >= DMA_BYTES);

        {
            let out = dma_buf.as_mut_slice();
            for (i, &sample) in mono.iter().enumerate() {
                let bytes = sample.to_le_bytes();
                let base = i * 4;
                out[base..base + 2].copy_from_slice(&bytes);
                out[base + 2..base + 4].copy_from_slice(&bytes);
            }
        }

        #[cfg(feature = "qemu")]
        return Ok(dma_buf);

        #[cfg(not(feature = "qemu"))]
        {
            let tx = self.tx.take().ok_or(AudioError::TransmitterBusy)?;
            match tx.write(dma_buf) {
                Ok(transfer) => {
                    let (_result, tx, buf) = transfer.wait_async().await;
                    self.tx = Some(tx);
                    Ok(buf)
                }
                Err((_, tx, buf)) => {
                    // DMA setup failed; keep going silently rather than wedge the
                    // whole system on a transient audio error.
                    self.tx = Some(tx);
                    Ok(buf)
                }
            }
        }
    }
}
