//! Desktop Input Bridge and Runtime Adapter
//!
//! This crate provides the desktop environment for the NES emulator. It maps
//! desktop inputs (keyboard, gamepads) to emulator actions and manages the
//! audio/video rendering runtime adapter.

#[cfg(feature = "mcp-host")]
pub mod mcp_host;

/// Command abstraction representing specific desktop user actions.
pub mod actions;
/// Desktop application state and main loop.
pub mod app;
/// CLI argument parsing definitions.
pub mod args;
/// Platform audio synchronization and playback. Ensuring the nostalgic beeps and boops actually reach the human ear.
pub mod audio;
/// Manual save state handling and persistence.
pub mod manual_state;
/// GUI menu structure and rendering.
pub mod menu;
/// Diagnostic overlay rendering tools.
pub mod overlay;
/// Netplay client logic and state management.
pub mod netplay;
/// Real-Time Attack (speedrunning) utilities. Because playing the game isn't enough; you must scientifically measure how fast you can do it.
pub mod rta;
/// Live runtime cheat code manager.
pub mod session_cheats;
