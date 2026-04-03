with open('crates/nes-desktop/src/rta.rs', 'r') as f:
    code = f.read()

search = """    fn write_draft_report(
        &self,
        profiles_dir: &Path,
        candidates: &[DraftCandidate],
    ) -> Result<PathBuf, String> {"""
replace = """    fn write_draft_report(
        &self,
        profiles_dir: &Path,
        candidates: Vec<DraftCandidate>,
    ) -> Result<PathBuf, String> {"""

code = code.replace(search, replace)
with open('crates/nes-desktop/src/rta.rs', 'w') as f:
    f.write(code)
