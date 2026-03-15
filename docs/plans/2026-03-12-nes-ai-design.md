# nes-ai Design

## Goal

Add a new `nes-ai` crate that provides a Rust-only training stack for NES agents using Burn, starting with a narrow but real milestone: SMB World 1-1 control training from a fixed gameplay snapshot.

The crate should not reimplement host or emulator behavior. `nes-core` remains the deterministic emulator and replay substrate. `nes-ai` owns the RL-facing environment wrapper, observation encoding, action mapping, reward calculation, episode recording, and Burn model/training code.

## Initial Milestone

The first success criterion is not "play NES generally" and not "beat SMB 1-1 from cold boot." That path leads directly to sparse-reward nonsense.

The initial milestone is:

- Start every episode from a fixed Super Mario Bros. World 1-1 snapshot.
- Train an agent to exhibit useful locomotion control.
- Reward forward progress, survival, and continued movement.
- Export evaluation episodes as deterministic TAS movies.
- Replay those episodes through the existing tooling stack.

This milestone is intentionally scoped to prove the infrastructure:

- deterministic reset
- stable observation pipeline
- stable reward plumbing
- Burn training loop integration
- reusable episode artifacts

## Non-Goals For V1

- Cold-boot title-screen navigation
- Full-level completion as the first acceptance target
- Multi-game generalization at runtime
- Pixel-perfect imitation of existing external RL APIs
- Full remote streaming or live spectator infrastructure

V1 should prove that the training system works on one concrete task while being structured so SMB-specific logic can later be replaced by other game profiles.

## Architecture

`nes-ai` should sit directly on top of `nes-core`.

`nes-core` already provides the important hard parts:

- deterministic stepping
- framebuffer capture
- memory reads
- save/load snapshots
- state hashing
- command replay
- TAS movie recording and replay

`nes-ai` should add the missing ML layer:

- `env.rs`
  - environment reset/step API
- `actions.rs`
  - discrete action mapping
- `obs.rs`
  - pixel and feature extraction
- `reward.rs`
  - reward and termination logic
- `episode.rs`
  - TAS-backed rollout capture and metadata
- `model.rs`
  - Burn policy/value network
- `trainer.rs`
  - PPO rollout collection and optimization
- `config.rs`
  - profile and runtime settings
- `error.rs`
  - crate-local error types

This keeps emulator logic and ML logic sharply separated instead of producing a swollen kitchen-sink crate.

## Environment Model

The first environment should be `SmbControlEnv`.

Reset flow:

1. Load ROM identity metadata.
2. Restore a fixed World 1-1 gameplay snapshot.
3. Clear episode bookkeeping.
4. Prime the frame stack.
5. Return the initial observation.

Step flow:

1. Map the agent action id to a controller bitmask.
2. Apply controller state to `NesCore`.
3. Advance a small frame-skip window.
4. Capture the latest visual observation.
5. Decode the feature vector.
6. Compute reward and termination state.
7. Record the input into a `TasRecorder`.
8. Return the transition result.

This environment is SMB-specific in v1, but the crate should be structured around a profile boundary so future games can swap in different snapshot packs, feature decoders, and reward logic without rewriting the trainer.

## Observation Model

The initial observation should be hybrid.

Visual branch:

- grayscale
- downsampled from the NES framebuffer
- stacked across recent frames

A reasonable starting default is an `84x84x4` frame stack, but the exact shape should remain configurable.

Feature branch:

- forward progress proxy
- timer/lives or equivalent HUD-derived game state
- motion or velocity proxy
- grounded/jump-state proxy where available
- a few generic emulator counters if useful

The important design rule is that the feature vector should be profile-defined. SMB can use SMB-specific memory layout now, but the `nes-ai` crate should isolate that logic behind a game profile interface instead of baking SMB offsets into the generic trainer.

## Action Space

The initial action space should be a tiny discrete set:

- `noop`
- `right`
- `right_a`
- `a`
- `right_b`
- `right_a_b`

This is intentionally restrictive. It removes most invalid exploration early and makes the control milestone learnable. The action mapping should live in one place and produce controller bitmasks compatible with `SetControllerState`.

Later milestones can add expanded platformer actions or raw controller masks, but v1 should prefer learnability over theoretical purity.

## Reward And Termination

The v1 reward should be shaped, but not baroque.

Positive signals:

- forward progress
- staying alive
- sustained movement

Negative signals:

- death
- long stalls or idling
- backward drift if it is materially harmful

Termination conditions:

- death
- episode frame budget exhausted
- stall threshold exceeded

The reward system should be profile-driven. SMB can define its own decode and scoring rules, but the generic trainer should only consume a compact `RewardBreakdown` or equivalent typed result.

## Episode Artifacts And Playback

The canonical episode artifact should be a `TasMovie` plus metadata.

Metadata should include at least:

- source snapshot identifier
- ROM hash or identity
- cumulative reward
- episode length
- final `state_hash`
- trainer/eval configuration summary

Playback strategy:

- replay `TasMovie` directly in-process when possible
- export legacy macro text only as a compatibility derivative

This is important because macro scripts are lossy, while TAS movies already align with the deterministic replay surface in `nes-core`.

## Burn Training Stack

The initial training stack should stay small and deterministic:

- Burn
- CPU-first execution
- seeded runs
- PPO
- policy/value shared backbone

The model should have two branches:

- a small CNN for the stacked visual input
- an MLP for the feature vector

Those branches merge into:

- action logits
- value estimate

The first version should avoid more ambitious algorithms and infrastructure. No replay-heavy off-policy setup, no distributed rollout workers, and no GPU-first assumptions in the acceptance path.

## Verification

Before claiming the trainer works, `nes-ai` should lock down environment behavior with tests:

- reset from the fixed snapshot is deterministic
- action ids map to the expected controller states
- observation tensor shapes and ranges are stable
- reward behaves sensibly on scripted trajectories
- exported TAS episodes replay to the same final `state_hash`

One integration smoke test should run a short scripted or seeded rollout and verify:

- positive forward reward is achievable
- an episode artifact is written
- replay of the artifact is deterministic

Training validation should compare a short PPO run against a random policy on average forward progress. If the trained policy does not beat random, the environment plumbing is suspect.

## Expansion Path Beyond SMB

SMB is only the first profile, not the long-term product boundary.

To support more games later, the crate should grow around explicit profile seams:

- snapshot pack per game or task
- feature decoder per game
- reward function per game
- task-specific action space if needed

That means the generic interfaces should be designed now so SMB-specific details are contained instead of smeared across the whole crate.

The practical rule is simple:

- emulator state remains generic in `nes-core`
- task semantics live in `nes-ai` profiles
- training infrastructure stays reusable

That gives the project a path from "teach Mario to run right" to broader NES tasks without rebuilding the stack from scratch every time.
