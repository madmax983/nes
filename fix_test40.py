import re
import os

def fix_file():
    filepath = 'crates/nes-test-harness/src/bin/bbbradsmith_golden_capture.rs'
    with open(filepath, 'r') as f:
        content = f.read()

    new_content = content.replace('''    let _ = crossterm::execute!(
        stdout,
        crossterm::terminal::Clear(crossterm::terminal::ClearType::CurrentLine),
        crossterm::cursor::MoveToColumn(0)
    );''', 'clear_current_line(stdout);')

    helper = '''fn clear_current_line(stdout: &mut impl Write) {
    let _ = crossterm::execute!(
        stdout,
        crossterm::terminal::Clear(crossterm::terminal::ClearType::CurrentLine),
        crossterm::cursor::MoveToColumn(0)
    );
}'''

    new_content = new_content.replace('fn print_processing_progress', helper + '\n\nfn print_processing_progress')

    test_helper = '''
    #[test]
    fn clear_current_line_writes_to_stdout_with_ansi() {
        let mut buf = Vec::new();
        super::clear_current_line(&mut buf);
        let output = String::from_utf8(buf).expect("should output valid utf8");
        assert!(output.contains("\\x1b[2K\\x1b[1G"));
    }
'''

    new_content = new_content.replace('    #[test]\n    fn print_processing_progress_writes_to_stdout_with_ansi()', test_helper + '\n    #[test]\n    fn print_processing_progress_writes_to_stdout_with_ansi()')

    with open(filepath, 'w') as f:
        f.write(new_content)

fix_file()
