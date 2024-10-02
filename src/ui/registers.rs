use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::Stylize,
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Widget},
};
use tui_textarea::{Input, Key, TextArea};

use crate::{
    simulator::{Highlight, Instruction, Registers, Simulator},
    util::make_title,
};

#[derive(Copy, Clone)]
pub struct RegisterUI<'a> {
    pub registers: &'a Registers,
    pub instrs: &'a [Instruction],
    pub state: Option<&'a RegisterUIState>,
}

impl Widget for RegisterUI<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = make_title("Registers", self.state.is_some());

        let block = Block::bordered().title(title).border_set(border::ROUNDED);

        let mut lines = Vec::with_capacity(32);

        let mut textarea_draw = None;

        for i in 0..31 {
            if Some(i) == self.state.map(|x| x.selected) {
                if let Some(area) = self.state.and_then(|x| x.replacing.as_ref()) {
                    textarea_draw = Some((i, area));

                    lines.push(Line::from(vec![
                        format!("X{i:<2}").bold().red().underlined(),
                        ": ".bold().underlined(),
                    ]));
                } else {
                    lines.push(Line::from(vec![
                        format!("X{i:<2}").bold().red().underlined(),
                        format!(": {}", self.registers.get(i).unwrap())
                            .bold()
                            .underlined(),
                    ]));
                }
            } else {
                lines.push(Line::from(vec![
                    format!("X{i:<2}").bold().red(),
                    format!(": {}", self.registers.get(i).unwrap()).into(),
                ]));
            }
        }

        if self.state.map(|x| x.selected) == Some(31) {
            if let Some(area) = self.state.and_then(|x| x.replacing.as_ref()) {
                textarea_draw = Some((31, area));

                lines.push(Line::from(vec![
                    "PC ".bold().green().underlined(),
                    ": ".bold().underlined(),
                ]));
            } else {
                lines.push(Line::from(vec![
                    "PC ".bold().green().underlined(),
                    format!(": {}", self.registers.pc * 4).bold().underlined(),
                ]));
            }
        } else {
            lines.push(Line::from(vec![
                "PC ".bold().green(),
                format!(": {}", self.registers.pc * 4).into(),
            ]));
        }

        let text_left = Text::from(lines[..16].to_owned());
        let text_right = Text::from(lines[16..].to_owned());

        let inner = block.inner(area);

        let layout = Layout::default()
            .direction(ratatui::layout::Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .areas::<2>(inner);

        block.render(area, buf);

        let pick_layout = Layout::default()
            .direction(ratatui::layout::Direction::Horizontal)
            .constraints([
                Constraint::Length(2), // For dest/src indicator
                Constraint::Fill(1),
            ]);

        let layout_left = pick_layout.areas::<2>(layout[0]);
        let layout_right = pick_layout.areas::<2>(layout[1]);

        text_left.render(layout_left[1], buf);
        text_right.render(layout_right[1], buf);

        if let Some(instr) = self.instrs.get(self.registers.pc as usize) {
            for i in 0..16 {
                if let Some(high) = instr.is_reg_highlighted(i) {
                    let place = Layout::vertical([
                        Constraint::Length(i as u16),
                        Constraint::Length(1),
                        Constraint::Fill(1),
                    ])
                    .areas::<3>(layout_left[0])[1];

                    match high {
                        Highlight::Source => "<".green().bold().render(place, buf),
                        Highlight::Dest => ">".cyan().bold().render(place, buf),
                    }
                }
            }

            for i in 16..31 {
                let place_i = i - 16;

                if let Some(high) = instr.is_reg_highlighted(i) {
                    let place = Layout::vertical([
                        Constraint::Length(place_i as u16),
                        Constraint::Length(1),
                        Constraint::Fill(1),
                    ])
                    .split(layout_right[0])[1];

                    match high {
                        Highlight::Source => "<".green().bold().render(place, buf),
                        Highlight::Dest => ">".cyan().bold().render(place, buf),
                    }
                }
            }

            if let Some(_) = instr.highlighted_instr(self.registers.pc) {
                let place = Layout::vertical([
                    Constraint::Length(15 as u16),
                    Constraint::Length(1),
                    Constraint::Fill(1),
                ])
                .areas::<3>(layout_right[0])[1];

                ">".cyan().bold().render(place, buf);
            }
        }

        if let Some((idx, area)) = textarea_draw {
            let lr_layout = if idx >= 16 {
                layout_right[1]
            } else {
                layout_left[1]
            };

            let place = Layout::vertical([
                Constraint::Length(idx as u16 % 16),
                Constraint::Length(1),
                Constraint::Fill(1),
            ])
            .areas::<3>(lr_layout)[1];

            let place = Layout::horizontal([Constraint::Length(5), Constraint::Fill(1)])
                .areas::<2>(place)[1];

            area.render(place, buf);
        }
    }
}

pub struct RegisterUIState {
    selected: u8,
    pub replacing: Option<TextArea<'static>>,
}

impl RegisterUIState {
    pub fn new() -> Self {
        Self {
            selected: 0,
            replacing: None,
        }
    }

    pub fn handle(&mut self, input: Input, state: &mut Simulator) {
        match input {
            Input { key: Key::Esc, .. } => self.replacing = None,

            Input {
                key: Key::Enter, ..
            } => {
                let Some(replacement) = self.replacing.take() else {
                    return;
                };

                let text = &replacement.lines()[0];

                if let Ok(new_val) = text.trim().parse::<i128>() {
                    if self.selected != 31 {
                        state.registers.set(self.selected, new_val as u64).unwrap();
                    } else {
                        state.registers.pc = new_val as u64 / 4;
                    }
                }
            }

            Input {
                key: Key::Char('r'),
                ctrl: true,
                ..
            } => {
                if self.replacing.is_none() {
                    self.replacing = Some(TextArea::default())
                }
            }

            input if self.replacing.is_some() => {
                let Some(replacing) = &mut self.replacing else {
                    unreachable!();
                };

                replacing.input(input);
            }

            Input { key: Key::Up, .. } => self.selected = (self.selected + 31) % 32,
            Input { key: Key::Down, .. } => self.selected = (self.selected + 1) % 32,
            Input {
                key: Key::Right | Key::Left,
                ..
            } => self.selected = (self.selected + 16) % 32,

            _ => {}
        }
    }
}
