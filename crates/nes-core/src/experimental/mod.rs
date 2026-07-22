//! Experimental features, visualizers, and tools ("Nova" persona).
//!
//! This module contains additive, exploratory features that build upon the core emulator.
//! These include visualizers for memory, nametables, and execution flow, as well as
//! tools for spatial querying, hotspot profiling, and memory heatmaps. They are
//! gated behind the `nova` feature flag to keep the base emulator footprint small
//! and ensure that core accuracy isn't impacted by experimental R&D.

/// Export utilities for dumping audio streams.
pub mod audio_exporter;
/// Utility for discovering cheat codes by monitoring RAM changes.
pub mod cheat_finder;
/// Tracks emulator events for external logging and analysis.
pub mod event_tracker;
/// Visualizes the flow of CPU instructions and call graphs.
pub mod execution_graph;
/// Draws visual bounding boxes over OAM sprites on the framebuffer.
pub mod hitbox_visualizer;
/// Identifies the most frequently executed CPU instructions.
pub mod hotspot_profiler;
/// Generates a visual heatmap of accessed memory regions.
pub mod memory_heatmap;
/// Provides memory visualization utilities for GUI consumption.
pub mod memory_visualizer;
/// Renders nametable memory into visual representations.
pub mod nametable_viewer;
/// Performs spatial queries against Object Attribute Memory (OAM).
pub mod oam_spatial_query;
/// Maps zone events to controller commands to create an auto-playing bot.
pub mod spatial_bot;
/// Extracts sprite graphical data from memory for inspection.
pub mod sprite_extractor;
/// Applies color palette themes to the emulator output.
pub mod theme_filter;
/// Tracks when sprites enter defined screen zones over time.
pub mod zone_tracker;

/// Visualizes PPU state including pattern tables and nametables with scroll boundaries.
#[cfg(feature = "nova")]
pub mod ppu_visualizer;

/// Applies a spotlight effect around a specific sprite, darkening the rest of the screen.
#[cfg(feature = "nova")]
pub mod flashlight_mode;
