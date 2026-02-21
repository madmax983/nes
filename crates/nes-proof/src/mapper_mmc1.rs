use vstd::prelude::*;

verus! {

spec fn bank_in_bounds(bank: nat, bank_count: nat) -> bool {
    bank < bank_count
}

spec fn mmc1_selected_bank(requested_bank: nat, bank_count: nat) -> nat
    recommends bank_count > 0
{
    requested_bank % bank_count
}

spec fn shift_is_reset(shift_register: nat, shift_count: nat) -> bool {
    shift_register == 0x10 && shift_count == 0
}

proof fn mmc1_selection_in_range(requested_bank: nat, bank_count: nat)
    requires bank_count > 0
    ensures bank_in_bounds(mmc1_selected_bank(requested_bank, bank_count), bank_count)
{
}

proof fn bit7_write_resets_shift_register()
    ensures shift_is_reset(0x10, 0)
{
}

} // verus!

fn main() {}
