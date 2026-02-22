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

#[test]
fn controller_state_and_speed_are_queryable() {
    let mut core = NesCore::new();
    core.execute(Command::SetControllerState(0xA5)).unwrap();
    core.execute(Command::SetSpeed(1_500)).unwrap();

    match core.query(CoreQuery::EmulatorState) {
        QueryResult::EmulatorState(state) => {
            assert_eq!(state.controller_bits, 0xA5);
            assert_eq!(state.speed_permille, 1_500);
        }
        other => panic!("unexpected query result: {other:?}"),
    }

    assert_eq!(
        core.query(CoreQuery::FpsMilli),
        QueryResult::FpsMilli(90_000)
    );
}
