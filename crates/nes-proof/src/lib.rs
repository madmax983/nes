//! Verus proof specs and lemmas
//!
//! A foundational scaffold crate to host formal correctness proofs,
//! specifically leveraging the `verus` toolchain.

pub fn proof_crate_marker() -> &'static str {
    "proof-scaffold-ready"
}
