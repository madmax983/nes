import re
import os

def fix_file():
    filepath = 'crates/nes-mcp/src/bin/run_macro.rs'
    with open(filepath, 'r') as f:
        content = f.read()

    new_content = content.replace('''    let mut stdout = std::io::stdout();
    let _ = crossterm::execute!(
        stdout,
        crossterm::terminal::Clear(crossterm::terminal::ClearType::CurrentLine),
        crossterm::cursor::MoveToColumn(0)
    );''', 'clear_current_line(&mut std::io::stdout());')

    helper = '''fn clear_current_line(stdout: &mut impl std::io::Write) {
    let _ = crossterm::execute!(
        stdout,
        crossterm::terminal::Clear(crossterm::terminal::ClearType::CurrentLine),
        crossterm::cursor::MoveToColumn(0)
    );
}'''

    new_content = new_content.replace('fn run(', helper + '\n\nfn run(')

    test_helper = '''
    #[test]
    fn clear_current_line_writes_to_stdout_with_ansi() {
        let mut buf = Vec::new();
        super::clear_current_line(&mut buf);
        let output = String::from_utf8(buf).expect("should output valid utf8");
        assert!(output.contains("\\x1b[2K\\x1b[1G"));
    }
'''

    new_content = new_content.replace('    #[test]\n    fn run_reports_missing_rom_file_errors()', test_helper + '\n    #[test]\n    fn run_reports_missing_rom_file_errors()')

    with open(filepath, 'w') as f:
        f.write(new_content)

fix_file()
