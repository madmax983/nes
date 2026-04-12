**[Refactoring format_profile_names manual iteration in nes-desktop/src/rta.rs]
**Learning:** Found a manual index-based string concatenation loop (`for (i, profile) in profiles.enumerate()`) used to join profile names into a comma-separated string, adding boilerplate and reducing idiomatic clarity.
**Action:** Replaced the manual iteration with an idiomatic `Iterator` chain using `.map().collect::<Vec<_>>().join(", ")`, allowing us to eliminate the index checking and push operations while maintaining clean output formatting.
