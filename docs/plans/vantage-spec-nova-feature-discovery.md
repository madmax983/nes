# 🔭 Vantage: Spec for Nova Feature Discovery

## 👤 User Story
As an exploratory user or contributor, I want to clearly understand which feature flags are required for experimental demos (like `story_demo`), so that I don't encounter confusing compiler errors (e.g., `NarrativeGenerator` not found) when trying them out.

## 💼 Business Problem (So What?)
Friction during onboarding and feature discovery leads to developer churn. If experimental features fail to compile out of the box because of hidden feature flags, it wastes time and makes the project appear broken. Clearly surfacing the `nova` feature requirement eliminates this friction and improves the developer experience.

## 📈 Success Metrics
- Zero new user reports or issues regarding missing types for experimental demos.

## ✅ Acceptance Criteria
- The `README.md` must include a clear, prominent banner or section indicating that experimental tools and demos require the `nova` feature flag.
- All command-line examples for running experimental demos must explicitly include the `--features nova` argument.

## 🚫 Out of Scope
- Enabling the `nova` feature by default in the workspace.
- Refactoring the experimental code itself.
