use std::{
    collections::VecDeque,
    sync::{Arc, Mutex, mpsc},
};

use ansi_to_tui::IntoText;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::Stylize,
    text::{Line, Text},
    widgets::{Block, Borders, List, ListState},
};

pub(crate) fn draw_thread(trace_names: Arc<Mutex<VecDeque<Option<String>>>>, rx: mpsc::Receiver<Vec<u8>>) {
    let mut terminal = ratatui::init();

    let mut data: Vec<(String, Vec<Text<'static>>)> = Vec::new();

    loop {
        if let Ok(trace) = rx.recv() {
            let name = trace_names
                .lock()
                .unwrap()
                .pop_front()
                .expect("trace received but no label detected");

            let Some(name) = name else {
                continue;
            };

            let trace = if let Ok(trace) = String::from_utf8(trace) {
                if let Ok(trace) = trace.into_text() {
                    trace
                } else {
                    continue;
                }
            } else {
                continue;
            };

            get_or_insert(&mut data, trace, name);

            let counter = data.len();

            terminal
                .draw(|frame| {
                    let main_chunk = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints(vec![Constraint::Ratio(1, counter as u32); counter])
                        .split(frame.area());

                    for (index, (name, trace)) in data.iter().enumerate() {
                        let mut state = ListState::default();

                        state.select(Some(trace.len() as usize));

                        let block = Block::default()
                            .title(Line::from(name.to_string()).gray().bold().centered())
                            .borders(Borders::ALL);

                        let list = List::new(trace.clone()).block(block);

                        frame.render_stateful_widget(list, main_chunk[index], &mut state);
                    }
                })
                .expect("failed to draw");
        }
    }
}

fn get_or_insert(data: &mut Vec<(String, Vec<Text<'static>>)>, trace: Text<'static>, name: String) {
    if let Some(pos) = data.iter().position(|(k, _)| k == &name) {
        data[pos].1.push(trace);
        return;
    }

    data.push((name, vec![trace]));
}
