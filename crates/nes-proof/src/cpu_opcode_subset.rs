use vstd::prelude::*;

verus! {

spec fn supported_subset(opcode: nat) -> bool {
    opcode == 0xA9 || opcode == 0xA2 || opcode == 0xA0
        || opcode == 0xAA || opcode == 0x8A || opcode == 0x98
        || opcode == 0xE8 || opcode == 0xEA
}

spec fn opcode_len(opcode: nat) -> nat {
    if opcode == 0xA9 || opcode == 0xA2 || opcode == 0xA0 {
        2
    } else {
        1
    }
}

spec fn next_pc(pc: nat, opcode: nat) -> nat
    recommends pc < 0x1_0000, supported_subset(opcode)
{
    (pc + opcode_len(opcode)) % 0x1_0000
}

proof fn supported_opcodes_have_bounded_lengths(opcode: nat)
    requires supported_subset(opcode)
    ensures 1 <= opcode_len(opcode) <= 2
{
}

proof fn supported_subset_pc_advance_is_defined(pc: nat, opcode: nat)
    requires pc < 0x1_0000, supported_subset(opcode)
    ensures next_pc(pc, opcode) < 0x1_0000
{
}

proof fn immediate_subset_advances_two_bytes(pc: nat, opcode: nat)
    requires pc < 0x1_0000, opcode == 0xA9 || opcode == 0xA2 || opcode == 0xA0
    ensures next_pc(pc, opcode) == (pc + 2) % 0x1_0000
{
}

proof fn single_byte_subset_opcodes_advance_one_byte(pc: nat, opcode: nat)
    requires
        pc < 0x1_0000,
        opcode == 0xAA || opcode == 0x8A || opcode == 0x98 || opcode == 0xE8 || opcode == 0xEA
    ensures next_pc(pc, opcode) == (pc + 1) % 0x1_0000
{
}

} // verus!

fn main() {}
