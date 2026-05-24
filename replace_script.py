import re

with open("crates/nes-desktop/src/rta.rs", "r") as f:
    content = f.read()

pattern1 = r"""    if let Some\(second_match\) = match_iter\.next\(\) \{
        let mut conflict_profiles = vec\!\[first_match\.clone\(\), second_match\.clone\(\)\];
        conflict_profiles\.extend\(match_iter\.cloned\(\)\);
        let conflict = format_profile_names\(&conflict_profiles\);
        return Err\(format!\(
            "Multiple RTA profiles matched ROM hash \{rom_hash\}: \{conflict\}"
        \)\);
    \}

    let selected = first_match\.clone\(\);
    check_draft\(&selected\)\?;

    Ok\(ProfileSelection \{
        selected,
        source: ProfileSelectionSource::AutoByRomHash,
    \}\)
\}"""

replacement1 = r"""    let Some(second_match) = match_iter.next() else {
        let selected = first_match.clone();
        check_draft(&selected)?;
        return Ok(ProfileSelection {
            selected,
            source: ProfileSelectionSource::AutoByRomHash,
        });
    };

    let mut conflict_profiles = vec![first_match.clone(), second_match.clone()];
    conflict_profiles.extend(match_iter.cloned());
    let conflict = format_profile_names(&conflict_profiles);
    Err(format!(
        "Multiple RTA profiles matched ROM hash {rom_hash}: {conflict}"
    ))
}"""

content = re.sub(pattern1, replacement1, content)

pattern2 = r"""            if let Some\(\(address, value, confidence\)\) = selected \{
                out\.push\(DraftCandidate \{
                    split_name: &split\.name,
                    address: address as u16,
                    value,
                    confidence,
                \}\);
            \}"""

replacement2 = r"""            let Some((address, value, confidence)) = selected else { continue };
            out.push(DraftCandidate {
                split_name: &split.name,
                address: address as u16,
                value,
                confidence,
            });"""

content = re.sub(pattern2, replacement2, content)


with open("crates/nes-desktop/src/rta.rs", "w") as f:
    f.write(content)
