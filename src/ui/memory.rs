use std::cell::Cell;

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::Stylize,
    symbols::border,
    text::{Line, Text},
    widgets::{block::Title, Block, Widget},
};
use tui_textarea::{Input, Key, TextArea};

use crate::{
    simulator::{Highlight, Instruction, Memory, Registers},
    util::{get_ranges, make_title},
};

#[derive(Copy, Clone)]
pub struct MemoryUI<'a> {
    pub memory: &'a Memory,
    pub instrs: &'a [Instruction],
    pub registers: &'a Registers,
    pub state: Option<&'a MemoryUIState>,
    pub persistent: &'a PersistentMemoryState,
}

impl Widget for MemoryUI<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = make_title("Memory", self.state.is_some());

        let block = Block::bordered().title(title).border_set(border::ROUNDED);

        let mem_interaction = self
            .instrs
            .get(self.registers.pc as usize)
            .and_then(|x| x.highlighted_mem(&self.registers));

        let interaction_idx = mem_interaction.map(|(x, _)| x);

        let selected_idx = self
            .state
            .map(|x| x.selected)
            .unwrap_or(self.persistent.selected.get() as u64);

        let to_view = get_ranges(
            &self.memory,
            1,
            interaction_idx.into_iter().chain([selected_idx]),
        );

        let mut lines = Vec::new();

        let max_height = block.inner(area).height as usize;

        let separator = Line::from(vec!["... zeros ...".into()]);

        let mut interaction_line_idx = None;
        let mut selected_line_idx = 0;

        for range in to_view {
            for x in range {
                let addr = x.wrapping_mul(8);

                if Some(x) == interaction_idx {
                    interaction_line_idx = Some(lines.len());
                }

                if x == selected_idx {
                    selected_line_idx = lines.len();

                    if self.state.and_then(|x| x.insertion.as_ref()).is_some() {
                        lines.push(Line::from(vec![
                            format!("{:<5}", addr).bold().red().underlined(),
                            format!(": ").underlined(),
                        ]));
                    } else {
                        if self.state.is_some() {
                            lines.push(Line::from(vec![
                                format!("{:<5}", addr).bold().red().underlined(),
                                format!(": {}", self.memory.get(addr).unwrap()).underlined(),
                            ]));
                        } else {
                            lines.push(Line::from(vec![
                                format!("{:<5}", addr).bold().red(),
                                format!(": {}", self.memory.get(addr).unwrap()).into(),
                            ]));
                        }
                    }
                } else {
                    lines.push(Line::from(vec![
                        format!("{:<5}", addr).bold().red(),
                        format!(": {}", self.memory.get(addr).unwrap()).into(),
                    ]));
                }
            }
            lines.push(separator.clone());
        }

        lines.pop();

        if lines.is_empty() {
            lines.push(Line::from(vec!["(all zeros)".into()]));
        }

        self.persistent.update(
            max_height,
            lines.len(),
            selected_line_idx,
            selected_idx as usize,
            3,
        );

        let to_remove = self.persistent.scroll_dist.get();

        let to_include = (max_height * 2).min(lines.len());

        let lines = &lines[to_remove..];

        let line_1;
        let mut line_2 = Vec::new();

        if to_include > max_height {
            line_1 = lines[..max_height].to_owned();
            line_2 = lines[max_height..to_include].to_owned();
            if to_include < lines.len() {
                line_2.pop();
                line_2.push(Line::from(vec!["-- Extra Below --".green()]));
            }
        } else {
            line_1 = lines.to_owned();
        }

        let text_left = Text::from(line_1);
        let text_right = Text::from(line_2);

        let inner = block.inner(area);

        let layout = Layout::default()
            .direction(ratatui::layout::Direction::Horizontal)
            .constraints(vec![
                Constraint::Length(2),
                Constraint::Percentage(50),
                Constraint::Length(2),
                Constraint::Percentage(50),
            ])
            .split(inner);

        block.render(area, buf);
        text_left.render(layout[1], buf);
        text_right.render(layout[3], buf);

        if let Some(MemoryUIState {
            insertion: Some(area),
            ..
        }) = &self.state
        {
            let line_idx = selected_line_idx - to_remove;

            if line_idx < to_include {
                let addr_remove = Layout::horizontal([Constraint::Length(7), Constraint::Fill(1)]);

                let guide_layout = Layout::vertical([
                    Constraint::Length((line_idx % max_height) as u16),
                    Constraint::Length(1),
                    Constraint::Fill(1),
                ]);

                let value_area = if line_idx < max_height {
                    addr_remove.areas::<2>(layout[1])[1]
                } else {
                    addr_remove.areas::<2>(layout[3])[1]
                };

                area.render(guide_layout.areas::<3>(value_area)[1], buf);
            }
        }

        if let Some(line_idx) = interaction_line_idx {
            if line_idx < to_include {
                let (_, highlight) = mem_interaction.unwrap();

                let span = match highlight {
                    Highlight::Source => "<".green().bold(),
                    Highlight::Dest => ">".cyan().bold(),
                };

                let area_layout = Layout::vertical([
                    Constraint::Length((line_idx % max_height) as u16),
                    Constraint::Length(1),
                    Constraint::Fill(1),
                ]);

                if line_idx < max_height {
                    span.render(area_layout.areas::<3>(layout[0])[1], buf);
                } else {
                    span.render(area_layout.areas::<3>(layout[2])[1], buf);
                }
            }
        }

        if let Some(input_area) = self.state.and_then(|x| x.line_selection.as_ref()) {
            let title = Title::from(" Goto ");
            let block = Block::bordered()
                .cyan()
                .title(title)
                .border_set(border::ROUNDED);

            let bottom_bits =
                Layout::vertical([Constraint::Fill(1), Constraint::Length(3)]).areas::<2>(inner)[1];

            let new_inner = block.inner(bottom_bits);

            block.render(bottom_bits, buf);

            input_area.render(new_inner, buf);
        }
    }
}

pub struct MemoryUIState {
    selected: u64,
    pub insertion: Option<TextArea<'static>>,
    pub line_selection: Option<TextArea<'static>>,
}

impl MemoryUIState {
    pub fn new(selected: usize) -> Self {
        Self {
            selected: selected as u64,
            insertion: None,
            line_selection: None,
        }
    }

    pub fn handle(&mut self, input: Input, memory: &mut Memory) {
        match input {
            Input { key: Key::Esc, .. } => {
                self.insertion = None;
                self.line_selection = None;
            }

            Input {
                key: Key::Enter, ..
            } if self.insertion.is_some() => {
                let area = self.insertion.take().unwrap();
                let text = area.lines()[0].parse::<i128>();

                if let Ok(val) = text {
                    memory.set(self.selected * 8, val as u64).unwrap();
                }
            }

            Input {
                key: Key::Enter, ..
            } if self.line_selection.is_some() => {
                let area = self.line_selection.take().unwrap();
                let text = area.lines()[0].parse::<i128>();

                if let Ok(val) = text {
                    self.selected = (val / 8) as u64;
                }
            }

            input if self.insertion.is_some() => {
                self.insertion.as_mut().unwrap().input(input);
            }

            input if self.line_selection.is_some() => {
                self.line_selection.as_mut().unwrap().input(input);
            }

            Input { key: Key::Up, .. } => self.selected = self.selected.saturating_sub(1),

            Input { key: Key::Down, .. } => self.selected = self.selected.wrapping_add(1),

            Input {
                key: Key::Char('g'),
                ..
            } => {
                self.line_selection = Some(TextArea::default());
            }

            Input {
                key: Key::Char('r'),
                ctrl: true,
                ..
            } => {
                self.insertion = Some(TextArea::default());
            }

            _ => {}
        }
    }
}

pub struct PersistentMemoryState {
    scroll_dist: Cell<usize>,
    pub selected: Cell<usize>,
}

impl PersistentMemoryState {
    pub fn new() -> Self {
        Self {
            scroll_dist: Cell::new(0),
            selected: Cell::new(0),
        }
    }

    pub fn update(
        &self,
        max_height: usize,
        len: usize,
        line_selected: usize,
        real_selected: usize,
        around_selected: usize,
    ) {
        self.selected.set(real_selected);

        let max_len = max_height * 2;
        if len <= max_len {
            self.scroll_dist.set(0);
            return;
        }

        let mut cur_scroll = self.scroll_dist.get();

        // we encounter:
        // -----[ window ]
        // ------------- (data)
        // And need to shift window left.
        if len - max_len < cur_scroll {
            self.scroll_dist.set(cur_scroll - (len - max_len));
            cur_scroll = self.scroll_dist.get();
        }

        let last_visible_elem = (line_selected + around_selected).min(len);
        let first_visible_elem = line_selected.saturating_sub(around_selected);

        if first_visible_elem < cur_scroll {
            self.scroll_dist.set(first_visible_elem);
        } else if last_visible_elem > cur_scroll + max_len {
            cur_scroll += last_visible_elem - (cur_scroll + max_len);
            self.scroll_dist.set(cur_scroll);
        }
    }
}
