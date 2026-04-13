//! Desktop Input Bridge and Runtime Adapter
//!
//! This crate provides the desktop environment for the NES emulator. It maps
//! desktop inputs (keyboard, gamepads) to emulator actions and manages the
//! audio/video rendering runtime adapter.

pub mod actions;
pub mod app;
pub mod args;
pub mod audio;
pub mod manual_state;
pub mod menu;
pub mod overlay;
pub mod rta;
pub mod session_cheats;
