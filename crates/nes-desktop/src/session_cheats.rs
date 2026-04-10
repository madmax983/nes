//! The player's runtime toolbox for bending the rules of the NES universe.
//!
//! This module maintains a local inventory of active and inactive Game Genie
//! codes during a single play session. Instead of forcing users to delete codes
//! they temporarily want to bypass, this manager tracks a discrete `enabled` flag
//! for each entry. When codes are sent to the core emulator, only the activated
//! subset is applied.
//!
//! Because `SessionCheats` validates raw code strings through the core `CheatCode`
//! parser immediately upon addition, it guarantees that its inventory never holds
//! syntactically malformed Game Genie commands.

use nes_core::{CheatCode, CheatCodeError};

/// One session-local cheat entry managed by the desktop frontend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionCheat {
    /// The normalized Game Genie string (e.g., "GOSSIP").
    pub raw_code: String,
    /// Whether this cheat is currently active in the core.
    pub enabled: bool,
}

/// Errors raised while mutating the session cheat list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionCheatError {
    /// The cheat code string was not a valid Game Genie sequence.
    InvalidCode(CheatCodeError),
    /// An operation was attempted on an index that does not exist in the session list.
    MissingIndex {
        /// The requested index.
        index: usize,
        /// The total number of entries.
        len: usize,
    },
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
    /// Creates an empty session cheat list.
    ///
    /// ## Examples
    ///
    /// ```
    /// use nes_desktop::session_cheats::SessionCheats;
    /// let cheats = SessionCheats::new();
    /// assert!(cheats.is_empty());
    /// ```
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Parses and initializes a session cheat list from multiple raw Game Genie strings.
    ///
    /// ## Examples
    ///
    /// ```
    /// use nes_desktop::session_cheats::SessionCheats;
    /// let codes = vec!["GOSSIP".to_owned()];
    /// let cheats = SessionCheats::from_raw_codes(&codes).unwrap();
    /// assert_eq!(cheats.len(), 1);
    /// ```
    pub fn from_raw_codes(raw_codes: &[String]) -> Result<Self, SessionCheatError> {
        let mut cheats = Self::new();
        for raw_code in raw_codes {
            cheats.add(raw_code)?;
        }
        Ok(cheats)
    }

    /// Validates, normalizes, and adds a new cheat code to the session, enabling it by default.
    ///
    /// ## Examples
    ///
    /// ```
    /// use nes_desktop::session_cheats::SessionCheats;
    /// let mut cheats = SessionCheats::new();
    /// cheats.add("GOSSIP").unwrap();
    /// assert_eq!(cheats.entries()[0].raw_code, "GOSSIP");
    /// ```
    pub fn add(&mut self, raw_code: &str) -> Result<(), SessionCheatError> {
        let normalized = normalize_cheat_code(raw_code)?;
        self.entries.push(SessionCheat {
            raw_code: normalized,
            enabled: true,
        });
        Ok(())
    }

    /// Toggles the enabled state of the cheat code at the specified index.
    ///
    /// ## Examples
    ///
    /// ```
    /// use nes_desktop::session_cheats::SessionCheats;
    /// let mut cheats = SessionCheats::new();
    /// cheats.add("GOSSIP").unwrap();
    /// cheats.toggle(0).unwrap();
    /// assert_eq!(cheats.entries()[0].enabled, false);
    /// ```
    pub fn toggle(&mut self, index: usize) -> Result<(), SessionCheatError> {
        let len = self.entries.len();
        let entry = self
            .entries
            .get_mut(index)
            .ok_or(SessionCheatError::MissingIndex { index, len })?;
        entry.enabled = !entry.enabled;
        Ok(())
    }

    /// Removes and returns the cheat code at the specified index.
    ///
    /// ## Examples
    ///
    /// ```
    /// use nes_desktop::session_cheats::SessionCheats;
    /// let mut cheats = SessionCheats::new();
    /// cheats.add("GOSSIP").unwrap();
    /// let removed = cheats.remove(0).unwrap();
    /// assert_eq!(removed.raw_code, "GOSSIP");
    /// ```
    pub fn remove(&mut self, index: usize) -> Result<SessionCheat, SessionCheatError> {
        if index >= self.entries.len() {
            return Err(SessionCheatError::MissingIndex {
                index,
                len: self.entries.len(),
            });
        }
        Ok(self.entries.remove(index))
    }

    /// Evicts all stored codes, wiping the slate clean for the next playthrough.
    ///
    /// ## Examples
    ///
    /// ```
    /// use nes_desktop::session_cheats::SessionCheats;
    /// let mut cheats = SessionCheats::new();
    /// cheats.add("GOSSIP").unwrap();
    /// cheats.clear();
    /// assert!(cheats.is_empty());
    /// ```
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Exposes the immutable ledger of every active and inactive code in the session.
    ///
    /// ## Examples
    ///
    /// ```
    /// use nes_desktop::session_cheats::SessionCheats;
    /// let mut cheats = SessionCheats::new();
    /// cheats.add("GOSSIP").unwrap();
    /// assert_eq!(cheats.entries()[0].raw_code, "GOSSIP");
    /// ```
    #[must_use]
    pub fn entries(&self) -> &[SessionCheat] {
        &self.entries
    }

    /// Extracts an array of just the raw Game Genie strings that are currently switched on.
    ///
    /// This output is ideal for directly feeding into the core emulator configuration.
    ///
    /// ## Examples
    ///
    /// ```
    /// use nes_desktop::session_cheats::SessionCheats;
    /// let mut cheats = SessionCheats::new();
    /// cheats.add("GOSSIP").unwrap();
    /// cheats.add("ZEXPYGLA").unwrap();
    /// cheats.toggle(1).unwrap(); // disable the second code
    /// assert_eq!(cheats.enabled_codes().collect::<Vec<_>>(), vec!["GOSSIP"]);
    /// ```
    ///
    /// **⚡ Bolt Optimization:** Returns an iterator instead of allocating a new `Vec<String>`.
    /// This removes unnecessary heap allocations and string cloning on the hot path
    /// when the emulator polls for active cheat codes during rendering/stepping.
    pub fn enabled_codes(&self) -> impl Iterator<Item = &str> {
        self.entries
            .iter()
            .filter(|entry| entry.enabled)
            .map(|entry| entry.raw_code.as_str())
    }

    /// Counts how many distinct cheat codes are being tracked, regardless of active status.
    ///
    /// ## Examples
    ///
    /// ```
    /// use nes_desktop::session_cheats::SessionCheats;
    /// let mut cheats = SessionCheats::new();
    /// cheats.add("GOSSIP").unwrap();
    /// assert_eq!(cheats.len(), 1);
    /// ```
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Verifies if the inventory is entirely empty.
    ///
    /// ## Examples
    ///
    /// ```
    /// use nes_desktop::session_cheats::SessionCheats;
    /// let cheats = SessionCheats::new();
    /// assert!(cheats.is_empty());
    /// ```
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
        assert_eq!(
            cheats.enabled_codes().collect::<Vec<_>>(),
            vec!["GOSSIP", "ZEXPYGLA", "GOSSIP"]
        );
    }

    #[test]
    fn toggle_and_remove_mutate_expected_entries() {
        let mut cheats =
            SessionCheats::from_raw_codes(&["GOSSIP".to_owned(), "ZEXPYGLA".to_owned()])
                .expect("seeded codes should validate");

        cheats.toggle(0).expect("toggle should succeed");
        assert!(!cheats.entries()[0].enabled);
        assert_eq!(cheats.enabled_codes().collect::<Vec<_>>(), vec!["ZEXPYGLA"]);

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

    #[test]
    fn clear_removes_all_entries() {
        let mut cheats =
            SessionCheats::from_raw_codes(&["GOSSIP".to_owned(), "ZEXPYGLA".to_owned()])
                .expect("seeded codes should validate");
        assert_eq!(cheats.len(), 2);
        cheats.clear();
        assert_eq!(cheats.len(), 0);
    }

    #[test]
    fn is_empty_returns_true_for_empty_list() {
        let mut cheats = SessionCheats::new();
        assert!(cheats.is_empty());
        cheats.add("GOSSIP").unwrap();
        assert!(!cheats.is_empty());
        cheats.clear();
        assert!(cheats.is_empty());
    }

    #[test]
    fn session_cheat_error_fmt() {
        use nes_core::CheatCodeError;
        let missing_index = SessionCheatError::MissingIndex { index: 5, len: 3 };
        assert_eq!(
            missing_index.to_string(),
            "cheat entry index 5 is out of bounds for 3 entries"
        );

        let invalid_code = SessionCheatError::InvalidCode(CheatCodeError::InvalidLength(4));
        assert_eq!(
            invalid_code.to_string(),
            CheatCodeError::InvalidLength(4).to_string()
        );
    }
}
