use vstd::prelude::*;

verus! {

/// A formalized model of the NES CPU state used for formal verification with Verus.
///
/// `CpuModel` abstracts the core registers of the 6502 processor into mathematical natural
/// numbers (`nat`). This mathematical representation enables the `vstd` verifier to logically
/// prove properties about the emulator's transitions without dealing with raw binary boundaries
/// immediately, instead enforcing bounds via specifications (e.g. `is_byte`).
pub struct CpuModel {
    pub a: nat,
    pub x: nat,
    pub y: nat,
    pub status: nat,
}

spec fn is_byte(v: nat) -> bool {
    v < 256
}

spec fn legal_status_bits(status: nat) -> bool {
    status < 256
}

spec fn model_is_legal(model: CpuModel) -> bool {
    is_byte(model.a)
        && is_byte(model.x)
        && is_byte(model.y)
        && legal_status_bits(model.status)
}

proof fn legal_model_preserves_bounds(model: CpuModel)
    requires model_is_legal(model)
    ensures
        is_byte(model.a),
        is_byte(model.x),
        is_byte(model.y),
        legal_status_bits(model.status),
{
    assert(is_byte(model.a));
    assert(is_byte(model.x));
    assert(is_byte(model.y));
    assert(legal_status_bits(model.status));
}

} // verus!

fn main() {}
