use {
    crate::draw::{DrawEvent, ScrollEvent},
    crossterm::{
        event::{
            EnableBracketedPaste, EnableFocusChange, EnableMouseCapture, Event, KeyCode, KeyEvent,
            KeyModifiers, MouseEventKind,
        },
        execute,
    },
    std::sync::{LazyLock, mpsc},
};

static EXIT_KEY: LazyLock<Event> =
    LazyLock::new(|| KeyCode::Char('c').into_event(KeyModifiers::CONTROL));

pub fn inputs_thread(tx: mpsc::Sender<DrawEvent>) {
    execute!(
        std::io::stdout(),
        EnableBracketedPaste,
        EnableFocusChange,
        EnableMouseCapture
    )
    .unwrap();

    loop {
        let event = crossterm::event::read().expect("Failed to read event");

        if event == *EXIT_KEY {
            ratatui::restore();
            std::process::exit(0);
        } else if let Event::Mouse(mouse) = event {
            let mut scroll = match mouse.kind {
                MouseEventKind::ScrollDown => -1,
                MouseEventKind::ScrollUp => 1,
                _ => {
                    continue;
                },
            };

            match mouse.modifiers {
                KeyModifiers::SHIFT => scroll *= 10,
                KeyModifiers::ALT => scroll *= i32::MAX,
                _ => {},
            }

            tx.send(DrawEvent::Mouse(ScrollEvent {
                scroll,
                column: mouse.column,
                row: mouse.row,
            }))
            .unwrap();
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
