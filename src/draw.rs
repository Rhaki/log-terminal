use {
    crate::index::{ContentIndex, PositionIdex, TabIndex, TypedVec},
    ansi_to_tui::IntoText,
    ratatui::{
        Frame,
        layout::{Constraint, Direction as LayoutDirection, Layout, Rect},
        style::{Style, Stylize},
        symbols::{self},
        text::{Line, Text},
        widgets::{
            Block, Borders, List, ListState, Scrollbar, ScrollbarOrientation, ScrollbarState, Tabs,
        },
    },
    std::{
        cmp::min,
        collections::{BTreeMap, VecDeque},
        sync::{Arc, LazyLock, Mutex, RwLock, mpsc},
    },
};

pub(crate) static MAX_LINES: LazyLock<Mutex<usize>> = LazyLock::new(|| Mutex::new(2_000));

pub(crate) enum Direction {
    Left,
    Right,
}

pub(crate) enum DrawEvent {
    Scroll(i32),
    ChangeSelect(Direction),
    MoveSelect(Direction),
    ChangeTab(Direction),
    Trace(Vec<u8>),
    Resize,
}

struct Row {
    log: String,
    lines: Option<Vec<Text<'static>>>,
}

impl Row {
    fn new(log: String) -> Self {
        Self { log, lines: None }
    }
}

struct Content {
    name: String,
    lines: RwLock<VecDeque<Row>>,
    offset: Offset,
    last_width: RwLock<u16>,
}

impl Content {
    fn scroll(&mut self, scroll: i32) {
        if scroll > 0 {
            self.offset
                .scroll_up(scroll, self.lines.read().unwrap().len());
        } else {
            self.offset
                .scroll_down(scroll, self.lines.read().unwrap().len());
        }
    }

    fn offset(&self) -> usize {
        self.offset.offset(self.lines.read().unwrap().len())
    }
}

#[derive(Default, PartialEq)]
struct TabPosition {
    tab_index: TabIndex,
    position_index: PositionIdex,
}

pub struct State {
    selected_tab: TabPosition,
    open_tabs: BTreeMap<TabIndex, PositionIdex>,
    tabs_position: TypedVec<TabIndex, TypedVec<PositionIdex, ContentIndex>>,
    contents: TypedVec<ContentIndex, Content>,
    trace_names: Arc<Mutex<VecDeque<Option<String>>>>,
    init: bool,
}

impl State {
    pub fn new(trace_names: Arc<Mutex<VecDeque<Option<String>>>>) -> Self {
        Self {
            selected_tab: TabPosition::default(),
            open_tabs: BTreeMap::new(),
            tabs_position: TypedVec::new(),
            contents: TypedVec::new(),
            trace_names,
            init: false,
        }
    }

    pub fn add_line(&mut self, line: String, name: String) {
        let Some(tab) = self.contents.iter_mut().find(|tab| tab.name == name) else {
            let add_selected = !self.init;

            if !self.init {
                self.init = true;
            }

            self.add_content(name, line, add_selected);

            return;
        };

        tab.lines.write().unwrap().push_back(Row::new(line));

        if tab.lines.read().unwrap().len() > *MAX_LINES.lock().unwrap() {
            tab.lines.write().unwrap().pop_front();
        }
    }

    fn get_selected_tab(&mut self) -> &mut Content {
        let index = self
            .tabs_position
            .get(self.selected_tab.tab_index)
            .unwrap()
            .get(self.selected_tab.position_index)
            .unwrap();

        self.contents.get_mut(*index).unwrap()
    }

    pub fn tab_count(&self) -> TabIndex {
        self.tabs_position.len()
    }

    fn add_content(&mut self, name: String, line: String, add_selected: bool) {
        self.contents.push(Content {
            name,
            lines: RwLock::new(VecDeque::from([Row::new(line)])),
            offset: Offset::new(),
            last_width: RwLock::new(0),
        });

        let content_index = self.contents.len().manipulate(|index| index - 1);

        self.add_tab(TabIndex(0), content_index, add_selected);
    }

    /// return true if the tab was removed
    fn remove_tab(
        &mut self,
        tab_index: TabIndex,
        position_index: PositionIdex,
        open_previous: bool,
    ) -> bool {
        if let Some(tabs) = self.tabs_position.get_mut(tab_index) {
            if tabs.len() > position_index {
                tabs.remove(position_index);
            }

            if tabs.is_empty() {
                self.tabs_position.remove(tab_index);

                // remove the tab from the open_tabs
                self.open_tabs.remove(&tab_index);

                let mut new_open_tabs = BTreeMap::new();

                for (k, v) in self.open_tabs.iter() {
                    if k > &tab_index {
                        new_open_tabs.insert(k.manipulate(|i| i.saturating_sub(1)), *v);
                    } else {
                        new_open_tabs.insert(*k, *v);
                    }
                }

                self.open_tabs = new_open_tabs;

                return true;
            } else if open_previous {
                let previous_position_index = position_index.manipulate(|i| i.saturating_sub(1));
                self.open_tabs.insert(tab_index, previous_position_index);

                return false;
            }
        }

        false
    }

    fn add_tab(&mut self, tab_index: TabIndex, content_index: ContentIndex, add_selected: bool) {
        let position_index = if let Some(tabs) = self.tabs_position.get_mut(tab_index) {
            tabs.push(content_index);

            tabs.len().manipulate(|i| i - 1)
        } else {
            if self.tabs_position.len() != tab_index {
                panic!("tab index is not valid");
            }

            self.tabs_position.push(TypedVec::from(vec![content_index]));

            PositionIdex(0)
        };

        if add_selected {
            self.select_tab(tab_index, position_index, false);
        }
    }

    fn select_tab(
        &mut self,
        tab_index: TabIndex,
        position_index: PositionIdex,
        close_previous: bool,
    ) {
        if close_previous {
            self.open_tabs.remove(&self.selected_tab.tab_index);
        }

        self.selected_tab = TabPosition {
            tab_index,
            position_index,
        };

        self.open_tabs.insert(tab_index, position_index);
    }

    fn get_current_content_index(&self) -> &ContentIndex {
        self.tabs_position
            .get(self.selected_tab.tab_index)
            .unwrap()
            .get(self.selected_tab.position_index)
            .unwrap()
    }

    fn rightest_tab(&self) -> TabPosition {
        let max_tab_index = self.tabs_position.len().manipulate(|index| index - 1);

        let max_position_index = self
            .tabs_position
            .get(max_tab_index)
            .unwrap()
            .len()
            .manipulate(|index| index - 1);

        TabPosition {
            tab_index: max_tab_index,
            position_index: max_position_index,
        }
    }

    fn change_select(&mut self, direction: Direction) -> Action {
        match direction {
            Direction::Left => {
                if self.selected_tab.tab_index == TabIndex(0)
                    && self.selected_tab.position_index == PositionIdex(0)
                {
                    return Action::Continue;
                }

                let (new_tab_index, new_position_index) = if self.selected_tab.position_index
                    == PositionIdex(0)
                {
                    let new_tab_index = self.selected_tab.tab_index.manipulate(|index| index - 1);

                    // Get last position index of tab_index -1

                    let last_position_index = self
                        .tabs_position
                        .get(new_tab_index)
                        .unwrap()
                        .len()
                        .manipulate(|index| index - 1);

                    (new_tab_index, last_position_index)
                } else {
                    (
                        self.selected_tab.tab_index,
                        self.selected_tab
                            .position_index
                            .manipulate(|index| index - 1),
                    )
                };

                // if the new tab index is different compared to the current tab index, we don't have to close the previous tab
                self.select_tab(
                    new_tab_index,
                    new_position_index,
                    new_tab_index == self.selected_tab.tab_index,
                );

                Action::Draw
            },
            Direction::Right => {
                let rightest_tab = self.rightest_tab();

                if rightest_tab == self.selected_tab {
                    return Action::Continue;
                }

                // check if in the current tab the selected position is the last one
                let current_tab = self.tabs_position.get(self.selected_tab.tab_index).unwrap();

                let (next_tab_index, next_position_index) = if current_tab
                    .len()
                    .manipulate(|index| index - 1)
                    == self.selected_tab.position_index
                {
                    let new_tab_index = self.selected_tab.tab_index.manipulate(|index| index + 1);

                    (new_tab_index, PositionIdex(0))
                } else {
                    (
                        self.selected_tab.tab_index,
                        self.selected_tab
                            .position_index
                            .manipulate(|index| index + 1),
                    )
                };

                // if the new tab index is different compared to the current tab index, we don't have to close the previous tab

                self.select_tab(
                    next_tab_index,
                    next_position_index,
                    next_tab_index == self.selected_tab.tab_index,
                );

                Action::Draw
            },
        }
    }

    fn move_select(&mut self, direction: Direction) -> Action {
        let current_tab_index = self.selected_tab.tab_index;

        match direction {
            Direction::Left => {
                if current_tab_index == TabIndex(0) {
                    return Action::Continue;
                }

                let next_tab = current_tab_index.manipulate(|index| index - 1);

                let current_content_index = *self.get_current_content_index();

                self.remove_tab(current_tab_index, self.selected_tab.position_index, true);

                self.add_tab(next_tab, current_content_index, true);
            },
            Direction::Right => {
                // If the current tab has only 1 element and it's the last tab, don't do anything
                let is_last_tab =
                    self.tabs_position.len() == current_tab_index.manipulate(|i| i + 1);

                if is_last_tab && *self.tabs_position.get(current_tab_index).unwrap().len() == 1 {
                    return Action::Continue;
                }

                let mut next_tab = current_tab_index.manipulate(|index| index + 1);

                let current_content_index = *self.get_current_content_index();

                let removed =
                    self.remove_tab(current_tab_index, self.selected_tab.position_index, true);

                if removed {
                    next_tab = next_tab.manipulate(|index| index - 1);
                }

                self.add_tab(next_tab, current_content_index, true);
            },
        }

        Action::Draw
    }

    fn change_tab(&mut self, direction: Direction) -> Action {
        let current_tab_index = self.selected_tab.tab_index;

        match direction {
            Direction::Left => {
                if current_tab_index == TabIndex(0) {
                    return Action::Continue;
                }

                let next_tab = current_tab_index.manipulate(|index| index - 1);

                self.selected_tab = TabPosition {
                    tab_index: next_tab,
                    position_index: *self.open_tabs.get(&next_tab).unwrap(),
                };

                Action::Draw
            },
            Direction::Right => {
                let is_last_tab =
                    self.tabs_position.len() == current_tab_index.manipulate(|i| i + 1);

                if is_last_tab {
                    return Action::Continue;
                }

                let next_tab = current_tab_index.manipulate(|index| index + 1);

                self.selected_tab = TabPosition {
                    tab_index: next_tab,
                    position_index: *self.open_tabs.get(&next_tab).unwrap(),
                };

                Action::Draw
            },
        }
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
                        .direction(LayoutDirection::Horizontal)
                        .constraints(vec![Constraint::Ratio(1, *tabs as u32); *tabs])
                        .split(frame.area());

                    for (index, content) in state.tabs_position.iter().enumerate() {
                        render_tab(TabIndex(index), content, main_chunk[index], &state, frame);
                    }
                })
                .unwrap();
        }
    }
}

fn render_tab(
    index: TabIndex,
    content: &TypedVec<PositionIdex, ContentIndex>,
    area: Rect,
    state: &State,
    frame: &mut Frame,
) {
    let mut header = vec![];

    let mut selected = None;

    let mut to_render = None;

    let render_index = state.open_tabs.get(&index).unwrap();

    for (position_index, content_index) in content.iter().enumerate() {
        let position_index = PositionIdex(position_index);
        let c = state.contents.get(*content_index).unwrap();
        header.push(c.name.clone());

        if position_index == *render_index {
            to_render = Some(content_index);
        }

        if state.selected_tab
            == (TabPosition {
                tab_index: index,
                position_index,
            })
        {
            selected = Some(position_index);
        }
    }

    let content = state.contents.get(*to_render.unwrap()).unwrap();

    let chunk = Layout::default()
        .direction(LayoutDirection::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area);

    let mut tabs = Tabs::new(header).highlight_style(Style::default());

    if let Some(selected) = selected {
        tabs = tabs
            .select(*selected)
            .highlight_style(Style::default().yellow().bold());
    }

    frame.render_widget(tabs, chunk[0]);

    render_content(content, selected.is_some(), chunk[1], frame);
}

fn render_content(tab: &Content, selected: bool, area: Rect, frame: &mut Frame) {
    let offset = tab.offset();

    let last_width = *tab.last_width.read().unwrap();

    let mut lines = tab.lines.write().unwrap();

    let trace_len = lines.len();

    let messages = lines.iter_mut().flat_map(|text| {
        if let (Some(lines), true) = (&text.lines, last_width == area.width) {
            return lines.clone();
        }

        let lines = textwrap::wrap(&text.log, area.width.saturating_sub(3) as usize)
            .into_iter()
            .filter_map(|text| {
                if text.is_empty() {
                    return None;
                } else {
                    text.as_ref().into_text().ok()
                }
            })
            .collect::<Vec<_>>();

        text.lines = Some(lines.clone());

        lines
    });

    if last_width != area.width {
        *tab.last_width.write().unwrap() = area.width;
    }

    // Render the list
    {
        let mut block = Block::default()
            .title(
                Line::from(format!(" {} ", tab.name))
                    .gray()
                    .bold()
                    .centered(),
            )
            .borders(Borders::ALL)
            .border_set(symbols::border::ROUNDED);

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

        let list = List::new(messages).block(block);

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
        DrawEvent::ChangeSelect(direction) => state.change_select(direction),
        DrawEvent::MoveSelect(change_tab_direction) => state.move_select(change_tab_direction),
        DrawEvent::ChangeTab(change_tab_direction) => state.change_tab(change_tab_direction),
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
        trace
    } else {
        return Action::Continue;
    };

    state.add_line(trace, name);

    Action::Draw
}
