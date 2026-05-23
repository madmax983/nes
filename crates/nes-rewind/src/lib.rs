//! Time Machine: player-facing rewind with anchor+delta compression.

/// Rewind cursor for querying historical state.
mod cursor;
/// State delta encoding and decoding utilities.
mod delta;
/// Keyframe placement policy and heuristic logic.
mod policy;
/// Ring-buffer timeline for storing compressed historical states.
mod timeline;
/// Asynchronous worker orchestrating state capturing and compression.
mod worker;

pub use cursor::{RewindCursor, RewindSpeed};
pub use delta::{ArrayDelta, FieldDelta, FrameDelta, apply_deltas};
pub use policy::KeyframePolicy;
pub use timeline::CompressedTimeline;
pub use worker::{TimeMachine, TimeMachineConfig, TimeMachineState};
