import re
import os

def fix_file():
    filepath = 'crates/nes-test-harness/src/bin/bbbradsmith_golden_capture.rs'
    with open(filepath, 'r') as f:
        content = f.read()

    new_content = content.replace('''fn print_processing_progress(stdout: &mut impl Write, rom_name: &str, color: Color) {
    let _ = write!(
        stdout,
        "\\r\\x1B[2K{}",
        format!("Processing {}...", rom_name).with(color)
    );
    let _ = stdout.flush();
}''', '''fn clear_current_line(stdout: &mut impl Write) {
    let _ = crossterm::execute!(
        stdout,
        crossterm::terminal::Clear(crossterm::terminal::ClearType::CurrentLine),
        crossterm::cursor::MoveToColumn(0)
    );
}

fn print_processing_progress(stdout: &mut impl Write, rom_name: &str, color: Color) {
    clear_current_line(stdout);
    let _ = write!(
        stdout,
        "{}",
        format!("Processing {}...", rom_name).with(color)
    );
    let _ = stdout.flush();
}''')

    new_content = new_content.replace('''    let _ = write!(stdout, "\\r\\x1B[2K");''', '    clear_current_line(stdout);')

    test_helper = '''    #[test]
    fn clear_current_line_writes_to_stdout_with_ansi() {
        let mut buf = Vec::new();
        super::clear_current_line(&mut buf);
        let output = String::from_utf8(buf).expect("should output valid utf8");
        assert!(output.contains("\\x1b[2K\\x1b[1G"));
    }
'''

    new_content = new_content.replace('    #[test]\n    fn print_processing_progress_writes_to_stdout_with_ansi() {', test_helper + '\n    #[test]\n    fn print_processing_progress_writes_to_stdout_with_ansi() {')
    new_content = new_content.replace('        assert!(output.contains("\\r\\x1B[2K"));', '        assert!(output.contains("\\x1b[2K\\x1b[1G"));')

    with open(filepath, 'w') as f:
        f.write(new_content)

fix_file()
