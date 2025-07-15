use {
    crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers},
    ratatui::widgets::{List, ListState},
};

#[test]
fn inputs() {
    let mut terminal = ratatui::init();

    let mut events = vec![];

    loop {
        let event = crossterm::event::read().expect("Failed to read event");

        if event == Event::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)) {
            break;
        }

        events.push(format!("{:?}", event));

        terminal
            .draw(|frame| {
                let block = List::new(events.clone());
                let mut state = ListState::default().with_selected(Some(events.len() - 1));
                frame.render_stateful_widget(block, frame.area(), &mut state);
            })
            .unwrap();
    }
}
