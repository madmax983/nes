//! Model Context Protocol (MCP) server integration for the NES emulator.

mod dispatch;
pub mod macro_engine;
pub mod mcp_host;
mod output;
pub mod tools;

pub use dispatch::{DispatchError, DispatchOutput, ToolParams, dispatch_tool};
pub use mcp_host::McpHost;
pub use output::{
    AudioChunk, FrameChunk, OutputMetadata, audio_chunk, frame_chunk, latest_output_metadata,
    publish_audio, publish_frame,
};
pub use tools::{ToolDefinition, tool_catalog};
