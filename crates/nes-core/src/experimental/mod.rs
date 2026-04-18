/// Export utilities for dumping audio streams.
pub mod audio_exporter;
/// Utility for discovering cheat codes by monitoring RAM changes.
pub mod cheat_finder;
/// Tracks emulator events for external logging and analysis.
pub mod event_tracker;
/// Visualizes the flow of CPU instructions and call graphs.
pub mod execution_graph;
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
/// Extracts sprite graphical data from memory for inspection.
pub mod sprite_extractor;
/// Applies color palette themes to the emulator output.
pub mod theme_filter;
/// Tracks when sprites enter defined screen zones over time.
pub mod zone_tracker;
