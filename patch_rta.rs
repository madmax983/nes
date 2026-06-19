        self.split_counter = self.split_counter.saturating_add(1);
        let elapsed_ms = self.elapsed(now).as_millis();
        let event = SplitEvent {
            name,
            source,
            frame,
            elapsed_ms,
        };
        self.split_events.push(event.clone());
        event
