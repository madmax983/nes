1. **Understand Goal**: Since I am "Vantage" 🔭, my task is to write a *spec* based on existing problems or user requests. I found `docs/ECHO_NOVA_REPORT.md` which highlights a problem: users try to run the `story_demo` but get a "NarrativeGenerator not found" error because they need to enable the `nova` feature.
2. **Draft the Spec**: I will create a spec addressing this gap. The spec should follow the format:
   - Title: "🔭 Vantage: Spec for Nova Feature Discovery"
   - User Story: "As a Developer or User, I want to be clearly informed about the `nova` feature requirement when exploring experimental features like `story_demo`, so that I don't waste time troubleshooting missing types like `NarrativeGenerator`."
   - Business Problem (So What?): "Hidden feature flags cause confusion and support requests. By clearly documenting the `nova` feature requirement, we improve the Developer Experience (DX) and reduce the barrier to entry for trying out our experimental narrative features."
   - Success Metrics: "Zero confusion reports about `NarrativeGenerator` not found when running the `story_demo`."
   - Acceptance Criteria:
     - The `README.md` must include a prominent banner or section clearly stating that `story_demo` (and `NarrativeGenerator`) requires compiling with the `--features nova` flag.
     - Any documentation referencing `story_demo` must explicitly mention the `nova` feature requirement.
   - Out of Scope: "Automatically enabling the `nova` feature by default."
3. **Write the file**: Create the spec at `docs/plans/vantage-spec-nova.md` or similar, or I can just use a bash command to create the file and use `submit`.
4. **Pre-commit**: I am Vantage, so I just create the spec document.
