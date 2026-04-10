import re
import os

def fix_file():
    filepath = 'crates/nes-test-harness/src/bin/bbbradsmith_golden_capture.rs'
    with open(filepath, 'r') as f:
        content = f.read()

    new_content = content.replace('''    #[test]
    fn clear_current_line(stdout: &mut impl Write) {''', '''    fn clear_current_line(stdout: &mut impl Write) {''')

    new_content = new_content.replace('''    fn clear_current_line(stdout: &mut impl Write) {
    let _ = crossterm::execute!(
        stdout,
        crossterm::terminal::Clear(crossterm::terminal::ClearType::CurrentLine),
        crossterm::cursor::MoveToColumn(0)
    );
}
    #[test]''', '''    #[test]''')

    with open(filepath, 'w') as f:
        f.write(new_content)

fix_file()
