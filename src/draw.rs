use {
    ansi_to_tui::IntoText,
    ratatui::{
        Frame,
        layout::{Constraint, Direction, Layout, Rect},
        style::{Style, Stylize},
        text::{Line, Text},
        widgets::{
            Block, Borders, List, ListState, Scrollbar, ScrollbarOrientation, ScrollbarState,
        },
    },
    std::{
        cmp::min,
        collections::VecDeque,
        sync::{Arc, LazyLock, Mutex, mpsc},
    },
};
pub(crate) static MAX_LINES: LazyLock<Mutex<usize>> = LazyLock::new(|| Mutex::new(2_000));

pub(crate) enum ChangeTabDirection {
    Left,
    Right,
}

pub(crate) enum DrawEvent {
    Scroll(i32),
    ChangeTab(ChangeTabDirection),
    Trace(Vec<u8>),
    Resize,
}

struct TabContent {
    name: String,
    lines: VecDeque<Text<'static>>,
    offset: Offset,
}

impl TabContent {
    fn scroll(&mut self, scroll: i32) {
        if scroll > 0 {
            self.offset.scroll_up(scroll, self.lines.len());
        } else {
            self.offset.scroll_down(scroll, self.lines.len());
        }
    }

    fn offset(&self) -> usize {
        self.offset.offset(self.lines.len())
    }
}

pub struct State {
    selected_tab: usize,
    tabs: Vec<TabContent>,
    trace_names: Arc<Mutex<VecDeque<Option<String>>>>,
}

impl State {
    pub fn new(trace_names: Arc<Mutex<VecDeque<Option<String>>>>) -> Self {
        Self {
            selected_tab: 0,
            tabs: Vec::new(),
            trace_names,
        }
    }

    pub fn add_line(&mut self, line: Text<'static>, name: String) {
        let Some(tab) = self.tabs.iter_mut().find(|tab| tab.name == name) else {
            self.tabs.push(TabContent {
                name,
                lines: VecDeque::from([line]),
                offset: Offset::new(),
            });

            return;
        };

        tab.lines.push_back(line);

        if tab.lines.len() > *MAX_LINES.lock().unwrap() {
            tab.lines.pop_front();
        }
    }

    fn get_selected_tab(&mut self) -> &mut TabContent {
        &mut self.tabs[self.selected_tab]
    }

    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }
}

pub(crate) enum Action {
    Draw,
    Continue,
}

struct Offset {
    offset: usize,
    enabled: bool,
}

impl Offset {
    pub fn new() -> Self {
        Self {
            offset: 0,
            enabled: false,
        }
    }

    fn scroll_up(&mut self, scroll: i32, trace_len: usize) {
        if !self.enabled {
            self.offset = trace_len;
            self.enabled = true;
        }

        self.offset = self.offset.saturating_sub(scroll.abs() as usize);
    }

    fn scroll_down(&mut self, scroll: i32, trace_len: usize) {
        if !self.enabled {
            return;
        }

        self.offset = min(self.offset.saturating_add(scroll.abs() as usize), trace_len);

        if self.offset == trace_len {
            self.enabled = false;
        }
    }

    fn offset(&self, trace_len: usize) -> usize {
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

    let mut state = State::new(trace_names);

    loop {
        if let Ok(trace) = rx.recv() {
            let action = handle_draw_event(&mut state, trace);

            if let Action::Continue = action {
                continue;
            }

            let tabs = state.tab_count();

            terminal
                .draw(|frame| {
                    let main_chunk = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints(vec![Constraint::Ratio(1, tabs as u32); tabs])
                        .split(frame.area());

                    for (index, tab) in state.tabs.iter().enumerate() {
                        render_tab(tab, index == state.selected_tab, main_chunk[index], frame);
                    }
                })
                .unwrap();
        }
    }
}

fn render_tab(tab: &TabContent, selected: bool, area: Rect, frame: &mut Frame) {
    let offset = tab.offset();
    let trace_len = tab.lines.len();

    // Render the list
    {
        let mut block = Block::default()
            .title(Line::from(tab.name.to_string()).gray().bold().centered())
            .borders(Borders::ALL);

        if offset != trace_len {
            block = block.title_bottom(
                Line::from(format!(" Scrolling: {} ", trace_len - offset))
                    .gray()
                    .left_aligned(),
            );
        }

        if selected {
            block = block.border_style(Style::default().yellow());
        }

        let list = List::new(tab.lines.clone()).block(block);

        let mut state = ListState::default().with_selected(Some(offset));

        frame.render_stateful_widget(list, area, &mut state);
    }

    // Render the scrollbar
    {
        let mut ss = ScrollbarState::new(trace_len).position(offset);

        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalLeft)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓")),
            area,
            &mut ss,
        );
    }
}

fn handle_draw_event(state: &mut State, event: DrawEvent) -> Action {
    match event {
        DrawEvent::Scroll(scroll_event) => {
            let tab = state.get_selected_tab();
            tab.scroll(scroll_event);
            Action::Draw
        },
        DrawEvent::Trace(trace) => on_trace_event(trace, state),
        DrawEvent::Resize => Action::Draw,
        DrawEvent::ChangeTab(direction) => {
            match direction {
                ChangeTabDirection::Left => {
                    state.selected_tab = state.selected_tab.saturating_sub(1);
                },
                ChangeTabDirection::Right => {
                    if state.selected_tab < state.tabs.len() - 1 {
                        state.selected_tab += 1;
                    } else {
                        return Action::Continue;
                    }
                },
            }

            Action::Draw
        },
    }
}

fn on_trace_event(trace: Vec<u8>, state: &mut State) -> Action {
    let name = state
        .trace_names
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

    state.add_line(trace, name);

    // get_or_insert(&mut state.data, trace, name);

    Action::Draw
}
