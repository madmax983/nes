#[cfg(test)]
mod tests {
    #[test]
    fn test_dummy() {
        let mut gilrs = gilrs::Gilrs::new().unwrap();
        // this doesn't help if there are no gamepads connected.
    }
}
