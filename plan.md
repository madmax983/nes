1. **Refactor `build_startup_table` in `crates/nes-desktop/src/main.rs`**
   - Modify the `crates/nes-desktop/src/main.rs` to extract an inline helper closure for `table.add_row`.
   - I will use `run_in_bash_session` to execute a python script that will replace all instances of `table.add_row(vec![Cell::new(NAME), Cell::new(VAL).fg(COLOR)])` with `add_row(NAME, &VAL, COLOR)`.
   - Specifically I will run:
     ```bash
     cat << 'PYTHON' > replace.py
     import re
     with open("crates/nes-desktop/src/main.rs", "r") as f:
         content = f.read()

     # We will do a targeted string replacement for build_startup_table
     # We know it starts around line 1372 and ends around 1480.
     start_idx = content.find("fn build_startup_table(")
     end_idx = content.find("table\n}", start_idx) + 7

     table_func = content[start_idx:end_idx]

     # Add the closure
     header_idx = table_func.find("]);\n") + 4

     closure = """
    let mut add_row = |name: &str, val: &dyn std::fmt::Display, color: TableColor| {
        table.add_row(vec![Cell::new(name), Cell::new(val).fg(color)]);
    };
"""
     table_func = table_func[:header_idx] + closure + table_func[header_idx:]

     # Replace manual calls
     # There are variations.
     table_func = table_func.replace(
"""    table.add_row(vec![
        Cell::new("ROM Path"),
        Cell::new(session.rom_path.display().to_string()).fg(TableColor::Green),
    ]);""",
"""    add_row("ROM Path", &session.rom_path.display().to_string(), TableColor::Green);"""
     )

     table_func = table_func.replace(
"""    table.add_row(vec![
        Cell::new("ROM Info"),
        Cell::new(format!(
            "Mapper {}, PRG {} bytes, reset vector ${:04X}",
            session.info.mapper_id, session.info.prg_rom_bytes, session.info.reset_pc
        ))
        .fg(TableColor::Green),
    ]);""",
"""    add_row(
        "ROM Info",
        &format!(
            "Mapper {}, PRG {} bytes, reset vector ${:04X}",
            session.info.mapper_id, session.info.prg_rom_bytes, session.info.reset_pc
        ),
        TableColor::Green,
    );"""
     )

     table_func = table_func.replace(
"""        table.add_row(vec![
            Cell::new("Config"),
            Cell::new(config_path.display().to_string()).fg(TableColor::Green),
        ]);""",
"""        add_row("Config", &config_path.display().to_string(), TableColor::Green);"""
     )

     table_func = table_func.replace(
"""    table.add_row(vec![
        Cell::new("Controls"),
        Cell::new(
            "keyboard Z=A, X=B, Enter=Start, RightShift=Select, Arrows=D-pad, R=Rewind, F5=Save Slot, F8=Load Slot, Esc=Menu",
        ).fg(TableColor::Green),
    ]);""",
"""    add_row(
        "Controls",
        &"keyboard Z=A, X=B, Enter=Start, RightShift=Select, Arrows=D-pad, R=Rewind, F5=Save Slot, F8=Load Slot, Esc=Menu",
        TableColor::Green,
    );"""
     )

     table_func = table_func.replace(
"""    table.add_row(vec![
        Cell::new("Menu"),
        Cell::new(if native_menu_supported() {
            "native menu bar + Esc overlay"
        } else {
            "Esc overlay only on this platform"
        })
        .fg(TableColor::Green),
    ]);""",
"""    add_row(
        "Menu",
        &if native_menu_supported() {
            "native menu bar + Esc overlay"
        } else {
            "Esc overlay only on this platform"
        },
        TableColor::Green,
    );"""
     )

     table_func = table_func.replace(
"""    table.add_row(vec![
        Cell::new("Gamepad"),
        Cell::new("face buttons=A/B, Start/Select, D-pad or left stick").fg(TableColor::Green),
    ]);""",
"""    add_row(
        "Gamepad",
        &"face buttons=A/B, Start/Select, D-pad or left stick",
        TableColor::Green,
    );"""
     )

     table_func = table_func.replace(
"""            table.add_row(vec![
                Cell::new("Step Mode"),
                Cell::new("frame").fg(TableColor::Green),
            ]);""",
"""            add_row("Step Mode", &"frame", TableColor::Green);"""
     )

     table_func = table_func.replace(
"""            table.add_row(vec![
                Cell::new("Step Mode"),
                Cell::new(format!("cpu ({steps} instructions/frame)")).fg(TableColor::Green),
            ]);""",
"""            add_row(
                "Step Mode",
                &format!("cpu ({steps} instructions/frame)"),
                TableColor::Green,
            );"""
     )

     table_func = table_func.replace(
"""        table.add_row(vec![
            Cell::new("Netplay"),
            Cell::new(format!(
                "relay={} room='{}' player={} delay={} rollback={} hash_every={}",
                netplay.relay_addr,
                netplay.room,
                netplay.player,
                netplay.input_delay_frames,
                if netplay.rollback_frames > 0 {
                    netplay.rollback_frames.to_string()
                } else {
                    "disabled".to_string()
                },
                netplay.hash_check_every
            ))
            .fg(TableColor::Green),
        ]);""",
"""        add_row(
            "Netplay",
            &format!(
                "relay={} room='{}' player={} delay={} rollback={} hash_every={}",
                netplay.relay_addr,
                netplay.room,
                netplay.player,
                netplay.input_delay_frames,
                if netplay.rollback_frames > 0 {
                    netplay.rollback_frames.to_string()
                } else {
                    "disabled".to_string()
                },
                netplay.hash_check_every
            ),
            TableColor::Green,
        );"""
     )

     table_func = table_func.replace(
"""        table.add_row(vec![
            Cell::new("RTA"),
            Cell::new(format!(
                "enabled profile='{}' calibrate={}",
                rta.profile_id(),
                rta.is_calibrating()
            ))
            .fg(TableColor::Green),
        ]);""",
"""        add_row(
            "RTA",
            &format!(
                "enabled profile='{}' calibrate={}",
                rta.profile_id(),
                rta.is_calibrating()
            ),
            TableColor::Green,
        );"""
     )

     table_func = table_func.replace(
"""            table.add_row(vec![
                Cell::new("Nova"),
                Cell::new("Auto Player Chaos Fuzzing Enabled"),
            ]);""",
"""            table.add_row(vec![
                Cell::new("Nova"),
                Cell::new("Auto Player Chaos Fuzzing Enabled"),
            ]); // Kept original to not break no-color styling if any, or we can just replace as well
            """
     )

     content = content[:start_idx] + table_func + content[end_idx:]

     with open("crates/nes-desktop/src/main.rs", "w") as f:
         f.write(content)
     PYTHON
     python3 replace.py
     rm replace.py
     ```

2. **Verify workspace**
   - Run `run_in_bash_session` to check `cargo fmt --all`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --workspace --all-targets --all-features`.

3. **Complete pre commit steps**
   - Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.

4. **Submit the Pull Request**
   - Target branch: `jules-9251055678621954092-4229af54`.
   - Title: "⚒️ Forge: Extract add_row closure in build_startup_table"
   - Description:
     - 🚮 Smell: Repetitive `table.add_row(vec![Cell::new(...), Cell::new(...)])` boilerplate in `build_startup_table`.
     - ✨ Solution: Extracted an `add_row` helper closure.
     - 🧼 Benefit: Improved readability and reduced code duplication (DRY).
     - 🛡️ Verification: Tests passed. No logic changed.
