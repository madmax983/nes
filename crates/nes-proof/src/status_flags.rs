use vstd::prelude::*;

verus! {

spec fn is_byte(v: nat) -> bool {
    v < 256
}

spec fn zero_flag_from_value(v: nat) -> bool {
    v == 0
}

spec fn negative_flag_from_value(v: nat) -> bool {
    128 <= v && v < 256
}

proof fn status_flags_are_legal_after_update(v: nat)
    requires is_byte(v)
    ensures
        zero_flag_from_value(v) == (v == 0),
        negative_flag_from_value(v) == (128 <= v),
{
    assert(zero_flag_from_value(v) == (v == 0));
    assert(negative_flag_from_value(v) == (128 <= v));
}

} // verus!

fn main() {}
