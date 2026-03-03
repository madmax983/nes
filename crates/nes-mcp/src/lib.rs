mod dispatch;
pub(crate) mod macro_engine;
mod output;
mod tools;

pub use dispatch::{DispatchError, DispatchOutput, ToolParams, dispatch_tool};
pub use macro_engine::execute_macro_script;
pub use output::{
    AudioChunk, FrameChunk, OutputMetadata, audio_chunk, frame_chunk, latest_output_metadata,
};
pub use tools::{ToolDefinition, tool_catalog};
