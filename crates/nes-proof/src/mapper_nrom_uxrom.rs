use vstd::prelude::*;

verus! {

spec fn bank_in_bounds(bank: nat, bank_count: nat) -> bool {
    bank < bank_count
}

spec fn uxrom_selected_bank(requested_bank: nat, bank_count: nat) -> nat
    recommends bank_count > 0
{
    requested_bank % bank_count
}

proof fn uxrom_selection_keeps_bank_in_range(requested_bank: nat, bank_count: nat)
    requires bank_count > 0
    ensures bank_in_bounds(uxrom_selected_bank(requested_bank, bank_count), bank_count)
{
}

proof fn nrom_bank_is_fixed()
    ensures bank_in_bounds(0, 1)
{
}

} // verus!

fn main() {}
