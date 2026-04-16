//! AI Control Training Infrastructure
//!
//! This crate provides the foundation for training reinforcement learning
//! agents (specifically using PPO via the `burn` framework) to play NES games.
//! It includes environment wrappers, discrete action spaces, reward models,
//! and utilities for running from specific save-state snapshots and emitting
//! TAS replay movies.

/// Defines the discrete action space, mapping abstract network outputs to concrete NES controller inputs. The puppet strings of the AI.
pub mod actions;
/// Configuration types for AI model training.
pub mod config;
/// The Gym-like environment wrapper. It provides the isolated reality where the agent lives, learns, and occasionally perishes.
pub mod env;
/// Tracks episode data, metrics, and rewards.
pub mod episode;
/// Standard error types for the AI pipeline.
pub mod error;
/// Burn neural network model definitions.
pub mod model;
/// Observation definitions bridging NES memory to tensors.
pub mod obs;
/// Core logic for defining AI evaluation profiles.
pub mod profile;
/// Concrete implementations of AI training profiles.
pub mod profiles;
/// Reward function structures and evaluation logic.
pub mod reward;
/// Memory state snapshot and restore utilities.
pub mod snapshot;
/// Training loop execution logic.
pub mod trainer;
