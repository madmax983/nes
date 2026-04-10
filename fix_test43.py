import re
import os

def fix_file():
    filepath = 'crates/nes-test-harness/src/bin/bbbradsmith_golden_capture.rs'
    with open(filepath, 'r') as f:
        content = f.read()

    new_content = content.replace('''fn clear_current_line(stdout: &mut impl std::io::Write) {
    let _ = crossterm::execute!(
        stdout,
        crossterm::terminal::Clear(crossterm::terminal::ClearType::CurrentLine),
        crossterm::cursor::MoveToColumn(0)
    );
}''', '''#[allow(dead_code)]
fn clear_current_line(stdout: &mut impl std::io::Write) {
    let _ = crossterm::execute!(
        stdout,
        crossterm::terminal::Clear(crossterm::terminal::ClearType::CurrentLine),
        crossterm::cursor::MoveToColumn(0)
    );
}''')

    with open(filepath, 'w') as f:
        f.write(new_content)

fix_file()
