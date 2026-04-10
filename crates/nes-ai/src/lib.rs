//! Reinforcement Learning tools for the NES emulator.
//!
//! This crate provides the infrastructure to train AI agents (using PPO via the `burn`
//! framework) to play NES games. It includes environment wrappers, action spaces,
//! reward calculation models, and snapshot/TAS movie serialization.

pub mod actions;
pub mod config;
pub mod env;
pub mod episode;
pub mod error;
pub mod model;
/// Observation processing and frame stacking.
pub mod obs;
/// The `TaskProfile` trait and related logic for extracting game features.
pub mod profile;
pub mod profiles;
pub mod reward;
/// Utilities for saving and loading emulator state snapshots.
pub mod snapshot;
/// The PPO training loop and agent execution logic.
pub mod trainer;
