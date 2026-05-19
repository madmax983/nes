sed -i 's/pub mod ascii_renderer;/#[cfg(feature = "nova")]\npub mod ascii_renderer;/' crates/nes-core/src/experimental/mod.rs
