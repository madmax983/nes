//! Terminal (ratatui) runtime adapter
//!
//! Provides a terminal-based interface for the NES emulator. Maps terminal
//! inputs to emulator actions and renders video frames to the console using
//! character block graphics and ANSI color codes.

/// Application core bridging raw input events to semantic emulator commands. Essential for mapping user intent to machine action.
pub mod app;
/// Rendering engine transforming raw NES pixels into blocky terminal magic. Crucial for the visual illusion of the TUI.
pub mod render;
