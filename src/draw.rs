use {
    ansi_to_tui::IntoText,
    ratatui::{
        layout::{Constraint, Direction, Layout, Position},
        style::Stylize,
        text::{Line, Text},
        widgets::{
            Block, Borders, List, ListState, Scrollbar, ScrollbarOrientation, ScrollbarState,
        },
    },
    std::{
        cmp::min,
        collections::{HashMap, VecDeque},
        sync::{Arc, Mutex, mpsc},
    },
};

pub(crate) struct ScrollEvent {
    pub scroll: i32,
    pub column: u16,
    pub row: u16,
}

pub(crate) enum DrawEvent {
    Mouse(ScrollEvent),
    Trace(Vec<u8>),
}

pub(crate) enum Action {
    Draw,
    Continue,
}

struct Offset {
    pub offset: usize,
    pub enabled: bool,
}

impl Offset {
    pub fn new(offset: usize) -> Self {
        Self {
            offset,
            enabled: true,
        }
    }

    pub fn scroll_up(&mut self, scroll: i32, trace_len: usize) {
        if !self.enabled {
            self.offset = trace_len;
            self.enabled = true;
        }

        self.offset = self.offset.saturating_sub(scroll.abs() as usize);
    }

    pub fn scroll_down(&mut self, scroll: i32, trace_len: usize) {
        self.offset = min(self.offset.saturating_add(scroll.abs() as usize), trace_len);

        if self.offset == trace_len {
            self.enabled = false;
        }
    }

    pub fn offset(&self, trace_len: usize) -> usize {
        if self.enabled {
            self.offset
        } else {
            trace_len
        }
    }
}

pub(crate) fn draw_thread(
    trace_names: Arc<Mutex<VecDeque<Option<String>>>>,
    rx: mpsc::Receiver<DrawEvent>,
) {
    let mut terminal = ratatui::init();
    let mut data: Vec<(String, Vec<Text<'static>>)> = Vec::new();
    let mut to_scroll: Option<ScrollEvent> = None;
    let mut scroll_offsets: HashMap<usize, Offset> = HashMap::new();

    loop {
        if let Ok(trace) = rx.recv() {
            let action = match trace {
                DrawEvent::Mouse(mouse) => {
                    to_scroll = Some(mouse);
                    Action::Draw
                },
                DrawEvent::Trace(trace) => on_trace_event(trace, &trace_names, &mut data),
            };

            if let Action::Continue = action {
                continue;
            }

            let counter = data.len();

            terminal
                .draw(|frame| {
                    let main_chunk = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints(vec![Constraint::Ratio(1, counter as u32); counter])
                        .split(frame.area());

                    for (index, (name, trace)) in data.iter().enumerate() {
                        let trace_len = trace.len() as usize;

                        // Handle the scroll event and calculate the offset
                        let offset = {
                            if let Some(scroll) = &to_scroll {
                                if main_chunk[index]
                                    .contains(Position::new(scroll.column, scroll.row))
                                {
                                    let offset = scroll_offsets
                                        .entry(index)
                                        .or_insert(Offset::new(trace_len));

                                    if scroll.scroll > 0 {
                                        offset.scroll_up(scroll.scroll, trace_len);
                                    } else {
                                        offset.scroll_down(scroll.scroll, trace_len);
                                    }
                                    to_scroll = None;
                                }
                            }

                            if let Some(offset) = scroll_offsets.get(&index) {
                                offset.offset(trace_len)
                            } else {
                                trace_len
                            }
                        };

                        // Render the list
                        {
                            let mut block = Block::default()
                                .title(Line::from(name.to_string()).gray().bold().centered())
                                .borders(Borders::ALL);

                            if offset != trace_len {
                                block = block.title_bottom(
                                    Line::from(format!(" Scrolling: {} ", trace_len - offset))
                                        .gray()
                                        .left_aligned(),
                                );
                            }

                            let list = List::new(trace.clone()).block(block);

                            let mut state = ListState::default().with_selected(Some(offset));

                            frame.render_stateful_widget(list, main_chunk[index], &mut state);
                        }

                        // Render the scrollbar
                        {
                            let mut ss = ScrollbarState::new(trace_len).position(offset);

                            frame.render_stateful_widget(
                                Scrollbar::new(ScrollbarOrientation::VerticalLeft)
                                    .begin_symbol(Some("↑"))
                                    .end_symbol(Some("↓")),
                                main_chunk[index],
                                &mut ss,
                            );
                        }
                    }
                })
                .expect("failed to draw");
        }
    }
}

fn on_trace_event(
    trace: Vec<u8>,
    trace_names: &Arc<Mutex<VecDeque<Option<String>>>>,
    data: &mut Vec<(String, Vec<Text<'static>>)>,
) -> Action {
    let name = trace_names
        .lock()
        .unwrap()
        .pop_front()
        .expect("trace received but no label detected");

    let Some(name) = name else {
        return Action::Continue;
    };

    let trace = if let Ok(trace) = String::from_utf8(trace) {
        if let Ok(trace) = trace.into_text() {
            trace
        } else {
            return Action::Continue;
        }
    } else {
        return Action::Continue;
    };

    get_or_insert(data, trace, name);

    Action::Draw
}

fn get_or_insert(data: &mut Vec<(String, Vec<Text<'static>>)>, trace: Text<'static>, name: String) {
    if let Some(pos) = data.iter().position(|(k, _)| k == &name) {
        data[pos].1.push(trace);
        return;
    }

    data.push((name, vec![trace]));
}
