#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_controller_state_default() {
        let state = ControllerState::default();
        assert_eq!(state.bits, 0);
        assert_eq!(state.shift, 0);
    }
}
