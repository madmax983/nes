//! Verus proof specs and lemmas
//!
//! A foundational scaffold crate to host formal correctness proofs,
//! specifically leveraging the `verus` toolchain.

/// Returns a static marker string indicating the proof crate is initialized.
///
/// This serves as a foundational link check and ensures the Verus toolchain has successfully compiled the scaffold.
///
/// ## Examples
///
/// ```
/// # use nes_proof::proof_crate_marker;
/// assert_eq!(proof_crate_marker(), "proof-scaffold-ready");
/// ```
pub fn proof_crate_marker() -> &'static str {
    "proof-scaffold-ready"
}
