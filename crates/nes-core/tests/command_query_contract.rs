use nes_core::{Command, CoreQuery, NesCore, QueryResult};

#[test]
fn boot_state_is_queryable_without_frontend() {
    let core = NesCore::new();
    let result = core.query(CoreQuery::EmulatorState);
    assert!(matches!(result, QueryResult::EmulatorState(_)));
}

#[test]
fn pause_and_resume_are_core_commands() {
    let mut core = NesCore::new();
    core.execute(Command::Pause).unwrap();
    assert!(core.is_paused());
    core.execute(Command::Resume).unwrap();
    assert!(!core.is_paused());
}
