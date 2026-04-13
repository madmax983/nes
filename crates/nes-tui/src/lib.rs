//! Terminal (ratatui) runtime adapter
//!
//! Provides a terminal-based interface for the NES emulator. Maps terminal
//! inputs to emulator actions and renders video frames to the console using
//! character block graphics and ANSI color codes.

pub mod app;
pub mod render;
