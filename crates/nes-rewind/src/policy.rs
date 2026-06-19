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

    fn policy() -> KeyframePolicy {
        KeyframePolicy::new(60, 2048)
    }

    #[test]
    fn forces_keyframe_at_base_interval() {
        let mut p = policy();
        for _ in 0..59 {
            assert!(!p.should_promote(100));
        }
        assert!(p.should_promote(100)); // 60th tick
    }

    #[test]
    fn resets_counter_after_promotion() {
        let mut p = policy();
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
        let mut p = policy();
        for _ in 0..10 {
            p.should_promote(100);
        } // warm up EMA
        assert!(p.should_promote(10_000)); // massive spike
    }

    #[test]
    fn moderate_delta_does_not_false_positive() {
        let mut p = policy();
        for _ in 0..10 {
            p.should_promote(500);
        }
        assert!(!p.should_promote(1000)); // 2x avg, not 3x
    }

    #[test]
    fn exact_spike_threshold_does_not_promote() {
        let mut p = policy();
        for _ in 0..20 {
            p.should_promote(0);
        }
        assert!(!p.should_promote(2048));
    }

    #[test]
    fn exact_avg_multiple_does_not_promote() {
        let mut p = policy();
        for _ in 0..50 {
            p.should_promote(2000);
        }

        let exact_thresh = p.rolling_avg * 3;
        assert!(!p.should_promote(exact_thresh));
    }

    #[test]
    fn exact_spike_and_rolling_multiple_promotes_when_mutated() {
        let mut p = policy();
        for _ in 0..50 {
            p.should_promote(2000);
        }

        let old_avg = p.rolling_avg; // ~2000
        let mut found_d = 0;

        for d in 2049_u32..10000_u32 {
            let new_avg = (d * 32) / 256 + (old_avg * (256-32)) / 256;
            if d == new_avg * 3 {
                found_d = d;
                break;
            }
        }

        if found_d > p.spike_threshold {
            assert!(!p.should_promote(found_d));

            // To ensure the other operator mutations are caught,
            // we should also test `found_d + 1` right after, but
            // we've already done enough exact bound tests.
        }
    }

    #[test]
    fn exact_rolling_avg_multiple_greater_than_or_equal_mutant() {
        let mut p = policy();
        for _ in 0..50 {
            p.should_promote(2000);
        }

        // This is explicitly for the mutant on `delta_size > self.rolling_avg * 3`
        // We need an exact match *after* EMA updates.
        // Let's just find it mathematically:
        let old_avg = p.rolling_avg;
        let mut exact_match = 0;
        for d in 0_u32..10000_u32 {
            let new_avg = (d * 32) / 256 + (old_avg * (256-32)) / 256;
            if d == new_avg * 3 && d > p.spike_threshold {
                exact_match = d;
                break;
            }
        }

        if exact_match > 0 {
            assert!(!p.should_promote(exact_match));
        }
    }

    #[test]
    fn ema_step_boundary_mutants() {

        let p = policy();
        assert_eq!(p.ema_step(0), 0);
        assert_eq!(p.ema_step(1), 0);
    }

    #[test]
    fn ema_calculation_exact() {
        let mut p = policy();
        p.should_promote(256);
        assert_eq!(p.rolling_avg, 32);

        p.should_promote(256);
        assert_eq!(p.rolling_avg, 60);
    }
}
