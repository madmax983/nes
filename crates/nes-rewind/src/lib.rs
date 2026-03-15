//! Time Machine: player-facing rewind with anchor+delta compression.

/// Rewind cursor for querying historical state.
pub mod cursor;
/// State delta encoding and decoding utilities.
pub mod delta;
/// Keyframe placement policy and heuristic logic.
pub mod policy;
/// Ring-buffer timeline for storing compressed historical states.
pub mod timeline;
/// Asynchronous worker orchestrating state capturing and compression.
pub mod worker;

pub use cursor::{RewindCursor, RewindSpeed};
pub use delta::{ArrayDelta, FieldDelta, FrameDelta};
pub use policy::KeyframePolicy;
pub use timeline::CompressedTimeline;
pub use worker::{TimeMachine, TimeMachineConfig, TimeMachineState};
