//! Model Context Protocol (MCP) server integration for the NES emulator.

mod dispatch;
/// Execution engine for deterministic automation macros.
pub mod macro_engine;
mod output;
/// Message protocol definitions for MCP communications.
pub mod protocol;
/// External tools integration mappings for the core.
pub mod tools;

pub use dispatch::{DispatchError, DispatchOutput, ToolParams, dispatch_tool};
pub use output::{
    AudioChunk, FrameChunk, OutputMetadata, audio_chunk, frame_chunk, latest_output_metadata,
    publish_audio, publish_audio_with, publish_frame, publish_frame_with,
};
pub use tools::{ToolDefinition, tool_catalog};
