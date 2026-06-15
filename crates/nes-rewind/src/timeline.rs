//! Anchor+delta ring buffer for the Time Machine rewind system.

use std::collections::VecDeque;

use nes_core::CoreSnapshot;

use crate::delta::FrameDelta;
use crate::policy::KeyframePolicy;

struct Keyframe {
    frame_id: u64,
    snapshot: CoreSnapshot,
}

/// Rolling history of emulator states as anchor keyframes + compressed deltas.
///
/// Stores full [`CoreSnapshot`] keyframes at adaptive intervals and
/// [`FrameDelta`]s between them. Old history is pruned automatically when
/// `max_frames` is exceeded.
pub struct CompressedTimeline {
    keyframes: VecDeque<Keyframe>,
    deltas: VecDeque<FrameDelta>,
    max_frames: u64,
    policy: KeyframePolicy,
    last_snapshot: Option<CoreSnapshot>,
    last_frame_id: Option<u64>,
}

impl CompressedTimeline {
    /// Create a new timeline with the given capacity and keyframe policy.
    pub fn new(max_frames: u64, policy: KeyframePolicy) -> Self {
        Self {
            keyframes: VecDeque::new(),
            deltas: VecDeque::new(),
            max_frames,
            policy,
            last_snapshot: None,
            last_frame_id: None,
        }
    }

    /// Push a new frame into the timeline.
    ///
    /// The first frame is always stored as a keyframe. Subsequent frames are
    /// stored as deltas unless [`KeyframePolicy`] promotes them to keyframes.
    pub fn push(&mut self, frame_id: u64, snapshot: CoreSnapshot) {
        if let Some(prev) = self.last_snapshot.take() {
            let mut delta = FrameDelta::compute(&prev, &snapshot);
            // Override with the caller-supplied frame_id — FrameDelta::compute
            // uses ppu.frame_counter which may lag behind the logical frame id.
            delta.frame_id = frame_id;
            let size = delta.compressed_size();

            if self.policy.should_promote(size) {
                self.keyframes.push_back(Keyframe {
                    frame_id,
                    snapshot: snapshot.clone(),
                });
            } else {
                self.deltas.push_back(delta);
            }
        } else {
            // First frame always anchors.
            self.keyframes.push_back(Keyframe {
                frame_id,
                snapshot: snapshot.clone(),
            });
        }

        self.last_snapshot = Some(snapshot);
        self.last_frame_id = Some(frame_id);
        self.prune();
    }

    /// Reconstruct the snapshot at `target_frame_id`.
    ///
    /// Finds the nearest keyframe at or before `target_frame_id`, then
    /// replays deltas forward. Returns `None` if the frame has been pruned,
    /// was never recorded, or is beyond the last recorded frame.
    #[must_use]
    pub fn reconstruct(&self, target_frame_id: u64) -> Option<CoreSnapshot> {
        // Reject frames we've never seen.
        if self.last_frame_id.is_none_or(|last| target_frame_id > last) {
            return None;
        }

        // Latest keyframe at or before target.
        let kf = self
            .keyframes
            .iter()
            .rev()
            .find(|kf| kf.frame_id <= target_frame_id)?;
        let mut snap = kf.snapshot.clone();

        if kf.frame_id == target_frame_id {
            return Some(snap);
        }

        // Replay deltas between keyframe and target.
        for delta in &self.deltas {
            if delta.frame_id > kf.frame_id && delta.frame_id <= target_frame_id {
                delta.apply(&mut snap);
            }
        }
        Some(snap)
    }

    /// Total number of recorded entries (keyframes + deltas).
    #[must_use]
    pub fn len(&self) -> usize {
        self.keyframes.len() + self.deltas.len()
    }

    /// Returns `true` if no frames have been recorded yet.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.keyframes.is_empty() && self.deltas.is_empty()
    }

    fn prune(&mut self) {
        while self.len() as u64 > self.max_frames {
            // Where does the second-oldest keyframe begin?
            let next_kf_id = self
                .keyframes
                .get(1)
                .map(|kf| kf.frame_id)
                .unwrap_or(u64::MAX);

            if self.deltas.front().is_some_and(|d| d.frame_id < next_kf_id) {
                // Advance the oldest keyframe by absorbing the first delta into
                // it in-place. This removes one delta (net -1) without breaking
                // the anchor chain.
                let delta = self.deltas.pop_front().unwrap();
                if let Some(oldest) = self.keyframes.front_mut() {
                    delta.apply(&mut oldest.snapshot);
                    oldest.frame_id = delta.frame_id;
                }
            } else {
                // No deltas anchored to oldest keyframe — drop it.
                self.keyframes.pop_front();
            }
        }
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn reconstruct_ignores_delta_at_kf_frame() {
        let mut tl = CompressedTimeline::new(10, crate::policy::KeyframePolicy::new(60, 2048));
        let snap1 = nes_core::NesCore::new().save_state();
        tl.last_frame_id = Some(11);
        tl.keyframes.push_back(Keyframe {
            frame_id: 10,
            snapshot: snap1.clone(),
        });

        let mut fd = FrameDelta::compute(&snap1, &snap1);
        fd.frame_id = 10;
        fd.fields.cpu_regs = Some(nes_core::cpu::CpuSnapshot {
            a: 100,
            ..snap1.cpu
        });
        tl.deltas.push_back(fd);

        let recon = tl.reconstruct(11).unwrap();
        assert_eq!(recon.cpu.a, 0);
    }

    #[test]
    fn test_is_empty_mutant() {
        let mut tl = CompressedTimeline::new(10, crate::policy::KeyframePolicy::new(60, 2048));
        let snap = nes_core::NesCore::new().save_state();
        tl.keyframes.push_back(Keyframe {
            frame_id: 1,
            snapshot: snap,
        });
        assert!(!tl.is_empty());
        let empty_tl = CompressedTimeline::new(10, crate::policy::KeyframePolicy::new(60, 2048));
        assert!(empty_tl.is_empty());
    }

    #[test]
    fn test_prune_mutants() {
        let mut tl2 = CompressedTimeline::new(2, crate::policy::KeyframePolicy::new(60, 2048));
        let snap1 = nes_core::NesCore::new().save_state();
        tl2.keyframes.push_back(Keyframe {
            frame_id: 10,
            snapshot: snap1.clone(),
        });
        tl2.keyframes.push_back(Keyframe {
            frame_id: 20,
            snapshot: snap1.clone(),
        });

        let mut fd = FrameDelta::compute(&snap1, &snap1);
        fd.frame_id = 20;
        tl2.deltas.push_back(fd);

        tl2.max_frames = 2;
        tl2.prune();

        assert_eq!(tl2.keyframes.front().unwrap().frame_id, 20);
        assert_eq!(tl2.deltas.len(), 1);

        let mut tl4 = CompressedTimeline::new(2, crate::policy::KeyframePolicy::new(60, 2048));
        let snap = nes_core::NesCore::new().save_state();
        tl4.keyframes.push_back(Keyframe {
            frame_id: 1,
            snapshot: snap.clone(),
        });
        tl4.keyframes.push_back(Keyframe {
            frame_id: 2,
            snapshot: snap.clone(),
        });
        tl4.keyframes.push_back(Keyframe {
            frame_id: 3,
            snapshot: snap.clone(),
        });
        tl4.prune();
        assert_eq!(tl4.len(), 2);

        let mut tl5 = CompressedTimeline::new(3, crate::policy::KeyframePolicy::new(60, 2048));
        tl5.keyframes.push_back(Keyframe {
            frame_id: 1,
            snapshot: snap.clone(),
        });
        tl5.keyframes.push_back(Keyframe {
            frame_id: 2,
            snapshot: snap.clone(),
        });
        tl5.keyframes.push_back(Keyframe {
            frame_id: 3,
            snapshot: snap.clone(),
        });
        tl5.prune();
        assert_eq!(tl5.len(), 3);

        let mut tl3 = CompressedTimeline::new(2, crate::policy::KeyframePolicy::new(60, 2048));
        tl3.keyframes.push_back(Keyframe {
            frame_id: 10,
            snapshot: snap1.clone(),
        });
        tl3.keyframes.push_back(Keyframe {
            frame_id: 20,
            snapshot: snap1.clone(),
        });
        let mut fd_clone = FrameDelta::compute(&snap1, &snap1);
        fd_clone.frame_id = 15;
        tl3.deltas.push_back(fd_clone);
        tl3.max_frames = 2;
    }

    #[test]
    fn reconstruct_strict_conditions() {
        let mut tl = CompressedTimeline::new(10, crate::policy::KeyframePolicy::new(60, 2048));
        let snap1 = nes_core::NesCore::new().save_state();
        let mut snap2 = snap1.clone();
        snap2.cpu.a = 42;
        snap2.ppu.frame_counter = 1;
        let mut snap3 = snap2.clone();
        snap3.cpu.a = 99;
        snap3.ppu.frame_counter = 2;

        tl.push(0, snap1.clone());
        tl.push(1, snap2.clone());
        tl.push(2, snap3.clone());

        let recon = tl.reconstruct(1).expect("should reconstruct");
        assert_eq!(recon.cpu.a, 42);
    }

    use super::*;
    use nes_core::{Command, NesCore};

    fn make_core() -> NesCore {
        let mut core = NesCore::new();
        core.load_cpu_bytes(0xC000, &[0xEA, 0x4C, 0x00, 0xC0]);
        core
    }

    fn policy() -> KeyframePolicy {
        KeyframePolicy::new(10, 2048)
    }

    #[test]
    fn empty_timeline_returns_none() {
        let tl = CompressedTimeline::new(300, policy());
        assert!(tl.is_empty());
        assert!(tl.reconstruct(0).is_none());
    }

    #[test]
    fn first_push_is_keyframe_and_reconstructs() {
        let core = make_core();
        let snap = core.save_state();
        let mut tl = CompressedTimeline::new(300, policy());
        tl.push(0, snap.clone());

        assert_eq!(tl.len(), 1);
        let got = tl.reconstruct(0).unwrap();
        assert_eq!(got.ppu.frame_counter, snap.ppu.frame_counter);
    }

    #[test]
    fn reconstruct_unknown_frame_returns_none() {
        let core = make_core();
        let mut tl = CompressedTimeline::new(300, policy());
        tl.push(0, core.save_state());
        assert!(tl.reconstruct(999).is_none());
    }

    #[test]
    fn push_prunes_when_over_capacity() {
        let core = make_core();
        let snap = core.save_state();
        let mut tl = CompressedTimeline::new(5, policy());
        for i in 0..10u64 {
            tl.push(i, snap.clone());
        }
        assert!(tl.len() <= 5);
        // Oldest frames should be gone.
        assert!(tl.reconstruct(0).is_none());
        // Most recent should still be available.
        assert!(tl.reconstruct(9).is_some());
    }

    #[test]
    fn reconstruct_after_step_frame_restores_earlier_state() {
        let mut core = make_core();
        let mut tl = CompressedTimeline::new(300, policy());

        let snap0 = core.save_state();
        tl.push(0, snap0);

        core.execute(Command::StepFrame).unwrap();
        let snap1 = core.save_state();
        let frame1 = snap1.ppu.frame_counter;
        tl.push(frame1, snap1);

        core.execute(Command::StepFrame).unwrap();
        let snap2 = core.save_state();
        let frame2 = snap2.ppu.frame_counter;
        tl.push(frame2, snap2);

        // Reconstruct frame 0 — should have frame_counter 0.
        let restored = tl.reconstruct(0).unwrap();
        assert_eq!(restored.ppu.frame_counter, 0);
    }
}
