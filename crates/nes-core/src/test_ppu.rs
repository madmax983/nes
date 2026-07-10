#[cfg(test)]
mod tests {
    use super::*;
    use crate::ppu::Ppu;

    #[test]
    fn should_not_panic_when_applying_due_live_bg_updates_with_empty_queue() {
        let mut ppu = Ppu::new();
        ppu.apply_due_live_bg_updates();
    }
}
