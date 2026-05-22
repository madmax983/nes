#[cfg(feature = "nova")]
pub struct AudioRumbleSimulator {
    pub intensity: f32,
    pub decay_rate: f32,
    pub threshold: i16,
}

#[cfg(feature = "nova")]
impl Default for AudioRumbleSimulator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "nova")]
impl AudioRumbleSimulator {
    pub fn new() -> Self {
        Self {
            intensity: 0.0,
            decay_rate: 0.9,
            threshold: 16000,
        }
    }

    pub fn process_chunk(&mut self, samples: &[i16]) {
        let mut peak: i16 = 0;
        for &sample in samples {
            let abs_sample = sample.saturating_abs();
            if abs_sample > peak {
                peak = abs_sample;
            }
        }

        if peak > self.threshold {
            let max_val = 32767.0;
            self.intensity = (peak as f32) / max_val;
        } else {
            self.intensity *= self.decay_rate;
        }
    }
}

#[cfg(feature = "nova")]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loud_sample_triggers_rumble() {
        let mut simulator = AudioRumbleSimulator::new();
        let chunk = vec![0, 0, 32000, 0];
        simulator.process_chunk(&chunk);
        assert!(simulator.intensity > 0.0);
    }

    #[test]
    fn test_rumble_intensity_decay() {
        let mut simulator = AudioRumbleSimulator::new();
        let loud_chunk = vec![0, 32000, 0];
        simulator.process_chunk(&loud_chunk);
        let initial = simulator.intensity;
        assert!(initial > 0.0);

        let quiet_chunk = vec![0, 0, 0];
        simulator.process_chunk(&quiet_chunk);
        assert!(simulator.intensity < initial);
    }
}
