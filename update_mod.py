with open("crates/nes-core/src/experimental/mod.rs", "r") as f:
    content = f.read()

new_content = content + """
/// Analyzes audio amplitude and applies visual distortions to the framebuffer.
#[cfg(feature = "nova")]
pub mod audio_reactive_filter;
"""

with open("crates/nes-core/src/experimental/mod.rs", "w") as f:
    f.write(new_content)
