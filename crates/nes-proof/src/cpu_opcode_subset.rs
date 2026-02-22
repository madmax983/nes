use vstd::prelude::*;

verus! {

spec fn supported_subset(opcode: nat) -> bool {
    opcode == 0x20 || opcode == 0x60
        || opcode == 0xA9 || opcode == 0xA2 || opcode == 0xA0
        || opcode == 0xAA || opcode == 0x8A || opcode == 0x98
        || opcode == 0xE8 || opcode == 0xEA
}

spec fn opcode_len(opcode: nat) -> nat {
    if opcode == 0x20 {
        3
    } else if opcode == 0xA9 || opcode == 0xA2 || opcode == 0xA0 {
        2
    } else {
        1
    }
}

spec fn linear_subset(opcode: nat) -> bool {
    opcode == 0xA9 || opcode == 0xA2 || opcode == 0xA0
        || opcode == 0xAA || opcode == 0x8A || opcode == 0x98
        || opcode == 0xE8 || opcode == 0xEA
}

spec fn next_pc_linear(pc: nat, opcode: nat) -> nat
    recommends pc < 0x1_0000, linear_subset(opcode)
{
    (pc + opcode_len(opcode)) % 0x1_0000
}

spec fn stack_push_count(opcode: nat) -> nat {
    if opcode == 0x20 {
        2
    } else {
        0
    }
}

spec fn stack_pull_count(opcode: nat) -> nat {
    if opcode == 0x60 {
        2
    } else {
        0
    }
}

proof fn supported_opcodes_have_bounded_lengths(opcode: nat)
    requires supported_subset(opcode)
    ensures 1 <= opcode_len(opcode) <= 3
{
}

proof fn linear_subset_pc_advance_is_defined(pc: nat, opcode: nat)
    requires pc < 0x1_0000, linear_subset(opcode)
    ensures next_pc_linear(pc, opcode) < 0x1_0000
{
}

proof fn immediate_subset_advances_two_bytes(pc: nat, opcode: nat)
    requires pc < 0x1_0000, opcode == 0xA9 || opcode == 0xA2 || opcode == 0xA0
    ensures next_pc_linear(pc, opcode) == (pc + 2) % 0x1_0000
{
}

proof fn single_byte_linear_subset_advances_one_byte(pc: nat, opcode: nat)
    requires
        pc < 0x1_0000,
        opcode == 0xAA || opcode == 0x8A || opcode == 0x98 || opcode == 0xE8 || opcode == 0xEA
    ensures next_pc_linear(pc, opcode) == (pc + 1) % 0x1_0000
{
}

proof fn jsr_has_two_pushes_and_len_three()
    ensures stack_push_count(0x20) == 2, opcode_len(0x20) == 3
{
}

proof fn rts_has_two_pulls_and_len_one()
    ensures stack_pull_count(0x60) == 2, opcode_len(0x60) == 1
{
}

} // verus!

fn main() {}
