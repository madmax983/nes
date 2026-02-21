use vstd::prelude::*;

verus! {

spec fn is_address(addr: int) -> bool {
    0 <= addr < 0x1_0000
}

spec fn map_region_id(addr: int) -> nat
    recommends is_address(addr)
{
    if addr < 0x2000 {
        0
    } else if addr < 0x4000 {
        1
    } else if addr < 0x4018 {
        2
    } else if addr < 0x4020 {
        3
    } else if addr < 0x6000 {
        4
    } else if addr < 0x8000 {
        5
    } else {
        6
    }
}

proof fn bus_map_total_and_bounded(addr: int)
    requires is_address(addr)
    ensures
        map_region_id(addr) <= 6,
        (addr < 0x2000) ==> map_region_id(addr) == 0,
        (0x2000 <= addr < 0x4000) ==> map_region_id(addr) == 1,
        (0x4000 <= addr < 0x4018) ==> map_region_id(addr) == 2,
        (0x4018 <= addr < 0x4020) ==> map_region_id(addr) == 3,
        (0x4020 <= addr < 0x6000) ==> map_region_id(addr) == 4,
        (0x6000 <= addr < 0x8000) ==> map_region_id(addr) == 5,
        (0x8000 <= addr < 0x1_0000) ==> map_region_id(addr) == 6,
{
}

} // verus!

fn main() {}
