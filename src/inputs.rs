use {
    crate::draw::{Direction, DrawEvent},
    crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers},
    std::sync::mpsc,
};

macro_rules! keys {
    ($($name:ident: $value:expr $(=> $modifiers:expr)?),* $(,)?) => {
        $(
            const $name: KeyEvent = KeyEvent::new(
                $value,
                keys!(@mod $($modifiers)?)
            );
        )*
    };

    (@mod $modifiers:expr) => {
        $modifiers
    };


    (@mod) => {
        KeyModifiers::NONE
    };
}

keys! {
    EXIT:            KeyCode::Char('c') => KeyModifiers::CONTROL,
    // Left
    SELECT_LEFT:     KeyCode::Left,
    MOVE_LEFT:       KeyCode::Left      => KeyModifiers::SHIFT,
    CHANGE_LEFT:     KeyCode::Char('b')      => KeyModifiers::ALT,
    // Right
    SELECT_RIGHT:    KeyCode::Right,
    MOVE_RIGHT:      KeyCode::Right     => KeyModifiers::SHIFT,
    CHANGE_RIGHT:    KeyCode::Char('f')     => KeyModifiers::ALT,
    // Scroll up
    SCROLL_UP:       KeyCode::Up,
    SCROLL_10_UP:    KeyCode::Up        => KeyModifiers::SHIFT,
    SCROLL_ALL_UP:   KeyCode::Up        => KeyModifiers::ALT,
    // Scroll down
    SCROLL_DOWN:     KeyCode::Down,
    SCROLL_10_DOWN:  KeyCode::Down      => KeyModifiers::SHIFT,
    SCROLL_ALL_DOWN: KeyCode::Down      => KeyModifiers::ALT,
}

pub fn inputs_thread(tx: mpsc::Sender<DrawEvent>) {
    loop {
        let event = crossterm::event::read().expect("Failed to read event");

        match event {
            Event::Key(key_event) => {
                // Exit
                if key_event == EXIT {
                    ratatui::restore();
                    std::process::exit(0);
                }
                // Left
                else if key_event == SELECT_LEFT {
                    tx.send(DrawEvent::ChangeSelect(Direction::Left)).unwrap();
                } else if key_event == MOVE_LEFT {
                    tx.send(DrawEvent::MoveSelect(Direction::Left)).unwrap();
                } else if key_event == CHANGE_LEFT {
                    tx.send(DrawEvent::ChangeTab(Direction::Left)).unwrap();
                }
                // Right
                else if key_event == SELECT_RIGHT {
                    tx.send(DrawEvent::ChangeSelect(Direction::Right)).unwrap();
                } else if key_event == MOVE_RIGHT {
                    tx.send(DrawEvent::MoveSelect(Direction::Right)).unwrap();
                } else if key_event == CHANGE_RIGHT {
                    tx.send(DrawEvent::ChangeTab(Direction::Right)).unwrap();
                }
                // Scroll up
                else if key_event == SCROLL_UP {
                    tx.send(DrawEvent::Scroll(1)).unwrap();
                } else if key_event == SCROLL_10_UP {
                    tx.send(DrawEvent::Scroll(10)).unwrap();
                } else if key_event == SCROLL_ALL_UP {
                    tx.send(DrawEvent::Scroll(i32::MAX)).unwrap();
                }
                // Scroll down
                else if key_event == SCROLL_DOWN {
                    tx.send(DrawEvent::Scroll(-1)).unwrap();
                } else if key_event == SCROLL_10_DOWN {
                    tx.send(DrawEvent::Scroll(-10)).unwrap();
                } else if key_event == SCROLL_ALL_UP {
                    tx.send(DrawEvent::Scroll(i32::MAX)).unwrap();
                } else if key_event == SCROLL_ALL_DOWN {
                    tx.send(DrawEvent::Scroll(-i32::MAX)).unwrap();
                }
            },

            Event::Resize(_, _) => tx.send(DrawEvent::Resize).unwrap(),
            _ => {},
        }
    }
}
