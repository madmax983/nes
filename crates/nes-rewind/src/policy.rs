/// Decides when to promote a full keyframe snapshot vs storing a delta.
///
/// Uses an Exponential Moving Average (EMA) of recent delta sizes to detect
/// spikes that warrant an early keyframe, plus a base interval guarantee.
#[derive(Debug, Clone)]
pub struct KeyframePolicy {
    base_interval: u64,
    spike_threshold: u32,
    frames_since_keyframe: u64,
    rolling_avg: u32,
}

impl KeyframePolicy {
    /// Creates a new `KeyframePolicy` with the given parameters.
    ///
    /// ## Arguments
    ///
    /// * `base_interval` - The guaranteed number of frames between keyframes,
    ///   assuming no delta spikes trigger an early keyframe.
    /// * `spike_threshold` - The heuristic threshold where a large frame delta
    ///   is considered complex enough to warrant a fresh keyframe immediately,
    ///   preventing expensive delta-chains.
    ///
    /// ## Examples
    ///
    /// ```
    /// use nes_rewind::policy::KeyframePolicy;
    ///
    /// let mut policy = KeyframePolicy::new(60, 4000);
    /// assert_eq!(policy.should_promote(100), false); // Still under interval
    /// assert_eq!(policy.should_promote(5000), true); // Triggered by spike
    /// ```
    pub fn new(base_interval: u64, spike_threshold: u32) -> Self {
        Self {
            base_interval,
            spike_threshold,
            frames_since_keyframe: 0,
            rolling_avg: 0,
        }
    }

    /// Returns true if a full keyframe should be stored this frame.
    ///
    /// Updates internal EMA. Resets counter on promotion.
    pub fn should_promote(&mut self, delta_size: u32) -> bool {
        self.frames_since_keyframe += 1;
        self.rolling_avg = self.ema_step(delta_size);

        if self.frames_since_keyframe >= self.base_interval {
            self.frames_since_keyframe = 0;
            return true;
        }

        if delta_size > self.spike_threshold && delta_size > self.rolling_avg * 3 {
            self.frames_since_keyframe = 0;
            return true;
        }

        false
    }

    fn ema_step(&self, new_value: u32) -> u32 {
        // alpha = 32/256 ~ 0.125 in Q8 fixed point
        const ALPHA_Q8: u32 = 32;
        let weighted_new = (new_value * ALPHA_Q8) >> 8;
        let weighted_old = (self.rolling_avg * (256 - ALPHA_Q8)) >> 8;
        weighted_new + weighted_old
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strict_greater_than_spike_threshold() {
        let mut p = KeyframePolicy::new(60, 2048);
        p.rolling_avg = 0;
        assert!(
            !p.should_promote(2048),
            "must not promote when exactly equal to spike threshold"
        );
    }

    #[test]
    fn test_strict_greater_than_rolling_avg_multiplier() {
        let mut p = KeyframePolicy::new(60, 2048);
        p.rolling_avg = 501;
        assert!(
            !p.should_promote(2100),
            "must not promote when exactly equal to 3x rolling avg"
        );
    }

    #[test]
    fn test_ema_step_mutant_addition() {
        let mut p = KeyframePolicy::new(60, 2048);
        p.rolling_avg = 1000;
        let res = p.ema_step(1600);
        assert_eq!(res, 1075);
        p.rolling_avg = 0;
        let res2 = p.ema_step(800);
        assert_eq!(res2, 100);
    }

    #[test]
    fn forces_keyframe_at_base_interval() {
        let mut p = KeyframePolicy::new(60, 2048);
        for _ in 0..59 {
            assert!(!p.should_promote(100));
        }
        assert!(p.should_promote(100)); // 60th tick
    }

    #[test]
    fn resets_counter_after_promotion() {
        let mut p = KeyframePolicy::new(60, 2048);
        for _ in 0..60 {
            p.should_promote(100);
        }
        for _ in 0..59 {
            assert!(!p.should_promote(100));
        }
        assert!(p.should_promote(100));
    }

    #[test]
    fn spike_triggers_early_promotion() {
        let mut p = KeyframePolicy::new(60, 2048);
        for _ in 0..10 {
            p.should_promote(100);
        } // warm up EMA
        assert!(p.should_promote(10_000)); // massive spike
    }

    #[test]
    fn moderate_delta_does_not_false_positive() {
        let mut p = KeyframePolicy::new(60, 2048);
        for _ in 0..10 {
            p.should_promote(500);
        }
        assert!(!p.should_promote(1000)); // 2x avg, not 3x
    }
}
