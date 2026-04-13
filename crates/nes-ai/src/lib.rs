//! AI Control Training Infrastructure
//!
//! This crate provides the foundation for training reinforcement learning
//! agents (specifically using PPO via the `burn` framework) to play NES games.
//! It includes environment wrappers, discrete action spaces, reward models,
//! and utilities for running from specific save-state snapshots and emitting
//! TAS replay movies.

pub mod actions;
pub mod config;
pub mod env;
pub mod episode;
pub mod error;
pub mod model;
pub mod obs;
pub mod profile;
pub mod profiles;
pub mod reward;
pub mod snapshot;
pub mod trainer;
