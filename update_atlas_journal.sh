echo "**Extract RuntimeConfig from main.rs**" >> .jules/atlas.md
echo "**Tangle:** The \`main.rs\` file in \`nes-desktop\` was bloated and contained configuration parsing (\`resolve_runtime_config\`, \`RuntimeConfig\`, \`CaptureConfig\`) mixed directly with the GUI application loop." >> .jules/atlas.md
echo "**Blueprint:** Extracted \`RuntimeConfig\`, \`StepMode\`, \`CaptureConfig\` and the \`resolve_runtime_config\` setup function into a separate internal \`config.rs\` module and updated \`main.rs\` imports to pull from it." >> .jules/atlas.md
