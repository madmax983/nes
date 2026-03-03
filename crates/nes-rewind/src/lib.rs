//! Time Machine: player-facing rewind with anchor+delta compression.

pub mod cursor;
pub mod delta;
pub mod policy;
pub mod timeline;
pub mod worker;

pub use cursor::{RewindCursor, RewindSpeed};
pub use delta::{ArrayDelta, FieldDelta, FrameDelta};
pub use policy::KeyframePolicy;
pub use timeline::CompressedTimeline;
pub use worker::{TimeMachine, TimeMachineConfig, TimeMachineState};
