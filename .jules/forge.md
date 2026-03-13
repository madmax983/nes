# Forge's Journal

**[nes-dsl Finalize Bug Fixed]
**Learning:** Found a panic-on-overwrite bug during forward reference resolution in `nes-dsl` caused by `self.write_resolved_byte(...)`.
**Action:** Always favor UPSERT (`insert(...)`) style resolution instead of strictly asserting `write_resolved_byte` matches over initial placeholder values when assembling.

**[nes-dsl handle_instruction Extraction]
**Learning:** `nes-dsl` had a massive `handle_instruction` function mixing syntax string matching and opcode generation.
**Action:** Extracting an `OperandSyntax` enum decoupling syntax parsing (`parse_operand_syntax`) from assembler logic flattens deeply nested code drastically.
