use std::sync::LazyLock;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

static EXIT_KEY: LazyLock<Event> =
    LazyLock::new(|| KeyCode::Char('c').into_event(KeyModifiers::CONTROL));

pub fn inputs_thread() {
    loop {
        let event = crossterm::event::read().expect("Failed to read event");

        if event == *EXIT_KEY {
            ratatui::restore();
            std::process::exit(0);
        }
    }
}

trait KeyCodeExt: Sized {
    fn into_event(self, modifier: KeyModifiers) -> Event;
}

impl KeyCodeExt for KeyCode {
    fn into_event(self, modifier: KeyModifiers) -> Event {
        Event::Key(KeyEvent::new(self, modifier))
    }
}
