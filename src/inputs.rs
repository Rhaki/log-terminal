use {
    crate::draw::{ChangeTabDirection, DrawEvent},
    crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers},
    std::sync::mpsc,
};

const EXIT: KeyEvent = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE);

const TAB_LEFT: KeyEvent = KeyEvent::new(KeyCode::Left, KeyModifiers::NONE);

const TAB_RIGHT: KeyEvent = KeyEvent::new(KeyCode::Right, KeyModifiers::NONE);

const SCROLL_UP: KeyEvent = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);

const SCROLL_10_UP: KeyEvent = KeyEvent::new(KeyCode::Up, KeyModifiers::SHIFT);

const SCROLL_ALL_UP: KeyEvent = KeyEvent::new(KeyCode::Up, KeyModifiers::ALT);

const SCROLL_DOWN: KeyEvent = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);

const SCROLL_10_DOWN: KeyEvent = KeyEvent::new(KeyCode::Down, KeyModifiers::SHIFT);

const SCROLL_ALL_DOWN: KeyEvent = KeyEvent::new(KeyCode::Down, KeyModifiers::ALT);

pub fn inputs_thread(tx: mpsc::Sender<DrawEvent>) {
    loop {
        let event = crossterm::event::read().expect("Failed to read event");

        match event {
            Event::Key(key_event) => {
                if key_event == EXIT {
                    ratatui::restore();
                    std::process::exit(0);
                } else if key_event == SCROLL_UP {
                    tx.send(DrawEvent::Scroll(1)).unwrap();
                } else if key_event == SCROLL_DOWN {
                    tx.send(DrawEvent::Scroll(-1)).unwrap();
                } else if key_event == TAB_LEFT {
                    tx.send(DrawEvent::ChangeTab(ChangeTabDirection::Left))
                        .unwrap();
                } else if key_event == TAB_RIGHT {
                    tx.send(DrawEvent::ChangeTab(ChangeTabDirection::Right))
                        .unwrap();
                } else if key_event == SCROLL_10_UP {
                    tx.send(DrawEvent::Scroll(10)).unwrap();
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
