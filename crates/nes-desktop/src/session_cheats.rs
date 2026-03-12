use nes_core::{CheatCode, CheatCodeError};

/// One session-local cheat entry managed by the desktop frontend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionCheat {
    pub raw_code: String,
    pub enabled: bool,
}

/// Errors raised while mutating the session cheat list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionCheatError {
    InvalidCode(CheatCodeError),
    MissingIndex { index: usize, len: usize },
}

impl std::fmt::Display for SessionCheatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidCode(err) => write!(f, "{err}"),
            Self::MissingIndex { index, len } => {
                write!(
                    f,
                    "cheat entry index {index} is out of bounds for {len} entries"
                )
            }
        }
    }
}

impl std::error::Error for SessionCheatError {}

impl From<CheatCodeError> for SessionCheatError {
    fn from(value: CheatCodeError) -> Self {
        Self::InvalidCode(value)
    }
}

/// Ordered current-session cheat list for the active ROM.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionCheats {
    entries: Vec<SessionCheat>,
}

impl SessionCheats {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn from_raw_codes(raw_codes: &[String]) -> Result<Self, SessionCheatError> {
        let mut cheats = Self::new();
        for raw_code in raw_codes {
            cheats.add(raw_code)?;
        }
        Ok(cheats)
    }

    pub fn add(&mut self, raw_code: &str) -> Result<(), SessionCheatError> {
        let normalized = normalize_cheat_code(raw_code)?;
        self.entries.push(SessionCheat {
            raw_code: normalized,
            enabled: true,
        });
        Ok(())
    }

    pub fn toggle(&mut self, index: usize) -> Result<(), SessionCheatError> {
        let len = self.entries.len();
        let entry = self
            .entries
            .get_mut(index)
            .ok_or(SessionCheatError::MissingIndex { index, len })?;
        entry.enabled = !entry.enabled;
        Ok(())
    }

    pub fn remove(&mut self, index: usize) -> Result<SessionCheat, SessionCheatError> {
        if index >= self.entries.len() {
            return Err(SessionCheatError::MissingIndex {
                index,
                len: self.entries.len(),
            });
        }
        Ok(self.entries.remove(index))
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    #[must_use]
    pub fn entries(&self) -> &[SessionCheat] {
        &self.entries
    }

    #[must_use]
    pub fn enabled_codes(&self) -> Vec<String> {
        self.entries
            .iter()
            .filter(|entry| entry.enabled)
            .map(|entry| entry.raw_code.clone())
            .collect()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

fn normalize_cheat_code(raw_code: &str) -> Result<String, SessionCheatError> {
    let normalized = raw_code.trim().to_ascii_uppercase();
    let _: CheatCode = normalized.parse()?;
    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::{SessionCheatError, SessionCheats};

    #[test]
    fn add_normalizes_lowercase_codes() {
        let mut cheats = SessionCheats::new();

        cheats.add("gossip").expect("valid code should add");

        assert_eq!(cheats.entries()[0].raw_code, "GOSSIP");
        assert!(cheats.entries()[0].enabled);
    }

    #[test]
    fn from_raw_codes_preserves_duplicate_order() {
        let cheats = SessionCheats::from_raw_codes(&[
            "GOSSIP".to_owned(),
            "ZEXPYGLA".to_owned(),
            "GOSSIP".to_owned(),
        ])
        .expect("seeded codes should validate");

        assert_eq!(cheats.len(), 3);
        assert_eq!(cheats.enabled_codes(), vec!["GOSSIP", "ZEXPYGLA", "GOSSIP"]);
    }

    #[test]
    fn toggle_and_remove_mutate_expected_entries() {
        let mut cheats =
            SessionCheats::from_raw_codes(&["GOSSIP".to_owned(), "ZEXPYGLA".to_owned()])
                .expect("seeded codes should validate");

        cheats.toggle(0).expect("toggle should succeed");
        assert!(!cheats.entries()[0].enabled);
        assert_eq!(cheats.enabled_codes(), vec!["ZEXPYGLA"]);

        let removed = cheats.remove(1).expect("remove should succeed");
        assert_eq!(removed.raw_code, "ZEXPYGLA");
        assert_eq!(cheats.len(), 1);
    }

    #[test]
    fn invalid_codes_are_rejected_before_mutation() {
        let mut cheats = SessionCheats::new();

        let err = cheats.add("BLAH").expect_err("invalid code should fail");

        assert!(matches!(err, SessionCheatError::InvalidCode(_)));
        assert!(cheats.is_empty());
    }

    #[test]
    fn missing_indices_report_bounds_errors() {
        let mut cheats = SessionCheats::new();
        let err = cheats.toggle(0).expect_err("missing entry should fail");
        assert_eq!(err, SessionCheatError::MissingIndex { index: 0, len: 0 });
    }
}
