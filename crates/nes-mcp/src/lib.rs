//! Model Context Protocol (MCP) server integration for the NES emulator.

mod dispatch;
pub mod experimental;
pub mod macro_engine;
mod output;
pub mod protocol;
pub mod tools;

pub use dispatch::{DispatchError, DispatchOutput, ToolParams, dispatch_tool};
pub use output::{
    AudioChunk, FrameChunk, OutputMetadata, audio_chunk, frame_chunk, latest_output_metadata,
    publish_audio, publish_frame,
};
pub use tools::{ToolDefinition, tool_catalog};
