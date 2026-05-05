#![allow(dead_code)]
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use std::io;
use std::time::{Duration, Instant};

pub trait EventSource {
    fn read_event(&mut self) -> Result<Event, String>;
    fn poll_event(&mut self, timeout: Duration) -> Result<bool, String>;
}

type NowFn = dyn Fn() -> Instant;
type SleepFn = dyn Fn(Duration);

pub struct CrosstermEventSource {
    pub poll_fn: Box<dyn Fn(Duration) -> io::Result<bool>>,
    pub read_fn: Box<dyn Fn() -> io::Result<Event>>,
}

impl CrosstermEventSource {
    pub fn new() -> Self {
        Self {
            poll_fn: Box::new(event::poll),
            read_fn: Box::new(event::read),
        }
    }

    #[cfg(test)]
    pub fn with_handlers(
        poll_fn: impl Fn(Duration) -> io::Result<bool> + 'static,
        read_fn: impl Fn() -> io::Result<Event> + 'static,
    ) -> Self {
        Self {
            poll_fn: Box::new(poll_fn),
            read_fn: Box::new(read_fn),
        }
    }
}

impl Default for CrosstermEventSource {
    fn default() -> Self {
        Self::new()
    }
}

impl EventSource for CrosstermEventSource {
    fn poll_event(&mut self, timeout: Duration) -> Result<bool, String> {
        (self.poll_fn)(timeout).map_err(|e| format!("Poll error: {e}"))
    }

    fn read_event(&mut self) -> Result<Event, String> {
        (self.read_fn)().map_err(|e| format!("Read error: {e}"))
    }
}

pub trait LoopTimer {
    fn wait_for_next_frame(&mut self, frame_interval: Duration);
    fn sleep_duration_for_frame(&self, frame_interval: Duration) -> Duration;
    fn now(&mut self) -> Instant;
}

pub struct SystemLoopTimer {
    pub last_frame_time: Instant,
    pub now_fn: Box<NowFn>,
    pub sleep_fn: Box<SleepFn>,
}

impl SystemLoopTimer {
    pub fn new() -> Self {
        Self {
            last_frame_time: Instant::now(),
            now_fn: Box::new(Instant::now),
            sleep_fn: Box::new(std::thread::sleep),
        }
    }

    #[cfg(test)]
    pub fn with_handlers(
        now_fn: impl Fn() -> Instant + 'static,
        sleep_fn: impl Fn(Duration) + 'static,
        start_time: Instant,
    ) -> Self {
        Self {
            last_frame_time: start_time,
            now_fn: Box::new(now_fn),
            sleep_fn: Box::new(sleep_fn),
        }
    }
}

impl Default for SystemLoopTimer {
    fn default() -> Self {
        Self::new()
    }
}

impl LoopTimer for SystemLoopTimer {
    fn now(&mut self) -> Instant {
        (self.now_fn)()
    }

    fn sleep_duration_for_frame(&self, frame_interval: Duration) -> Duration {
        let now = (self.now_fn)();
        let elapsed = now.saturating_duration_since(self.last_frame_time);

        if elapsed < frame_interval {
            frame_interval - elapsed
        } else {
            Duration::ZERO
        }
    }

    fn wait_for_next_frame(&mut self, frame_interval: Duration) {
        let sleep_duration = self.sleep_duration_for_frame(frame_interval);
        if sleep_duration > Duration::ZERO {
            (self.sleep_fn)(sleep_duration);
        }

        self.last_frame_time = (self.now_fn)();
    }
}

pub fn should_quit(key: KeyEvent) -> bool {
    key_pressed_state(key.kind) == Some(true)
        && matches!(key.code, KeyCode::Esc | KeyCode::Char('q'))
}

pub fn key_pressed_state(kind: KeyEventKind) -> Option<bool> {
    match kind {
        KeyEventKind::Press | KeyEventKind::Repeat => Some(true),
        KeyEventKind::Release => Some(false),
    }
}

pub fn key_is_pressed(kind: KeyEventKind) -> bool {
    matches!(kind, KeyEventKind::Press | KeyEventKind::Repeat)
}
