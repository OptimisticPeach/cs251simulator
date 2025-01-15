use color_eyre::eyre::Error;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::Stylize,
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Paragraph, Widget},
};
use tui_textarea::{Input, Key, TextArea};

use crate::{
    simulator::{Instruction, Memory, Registers, Simulator},
    util::make_title,
};

#[derive(Copy, Clone)]
pub struct InstructionUI<'a> {
    pub instrs: &'a [Instruction],
    pub registers: &'a Registers,
    pub memory: &'a Memory,
    pub pc: u64,
    pub state: Option<&'a InstructionUIState>,
}

impl Widget for InstructionUI<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = if let Some(InstructionUIState { text: Some(_), .. }) = self.state {
            make_title("Inserting", self.state.is_some())
        } else {
            make_title("Instructions", self.state.is_some())
        };

        let block = Block::bordered().title(title).border_set(border::ROUNDED);

        let idx_width = (self.instrs.len() as f32).log10().floor() as usize + 1;

        let mut lines = self
            .instrs
            .iter()
            .map(Instruction::get_line)
            .collect::<Vec<_>>();

        for (idx, line) in lines.iter_mut().enumerate() {
            let line_string = format!("{idx:<width$} ", width = idx_width);

            let line_number = if idx == self.pc as usize {
                if let Some(InstructionUIState {
                    text: Some(text), ..
                }) = self.state
                {
                    line.clear();

                    if text.lines()[0].parse::<Instruction>().is_err() {
                        line_string.red()
                    } else {
                        line_string.green()
                    }
                } else {
                    line_string.green()
                }
            } else {
                line_string.into()
            };

            line.insert(0, line_number);
        }

        let height = block.inner(area).height as usize;

        while lines.len() > height {
            lines.pop();
        }

        let lines = lines.into_iter().map(Line::from);

        let text = Text::from(lines.collect::<Vec<_>>());

        let instruction_to_explain = self.instrs.get(self.registers.pc as usize).map(|x| {
            if let Some(InstructionUIState {
                text: Some(text), ..
            }) = self.state
            {
                let parsed = text.lines()[0].parse::<Instruction>();

                match parsed {
                    Ok(x) => Ok(x),
                    Err(e) => Err(e.to_string()),
                }
            } else {
                Ok(x.clone())
            }
        });

        let height_explanation = match &instruction_to_explain {
            None => 0,
            Some(Ok(_)) => 4,
            Some(Err(e)) => 2 + e.lines().count(),
        };

        let vert_layout = Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(height_explanation as u16),
            ])
            .split(block.inner(area));

        let instrs_layout = Layout::default()
            .direction(ratatui::layout::Direction::Horizontal)
            .constraints([
                Constraint::Length(2), // selected instr, target instr
                Constraint::Fill(1),
            ])
            .split(vert_layout[0]);

        block.render(area, buf);

        Paragraph::new(text).render(instrs_layout[1], buf);

        if let Some(instr_or_err) = instruction_to_explain {
            let by_ref = instr_or_err.as_ref().map_err(|x| &**x);
            let explanation = InstructionExplanation {
                instr: by_ref,
                registers: self.registers,
                memory: self.memory,
            };

            explanation.render(vert_layout[1], buf);

            if let Some(target) = by_ref
                .ok()
                .and_then(|x| x.highlighted_instr(self.registers.pc))
            {
                let target_pos = Layout::default()
                    .direction(ratatui::layout::Direction::Vertical)
                    .constraints([
                        Constraint::Length(target as u16),
                        Constraint::Length(1),
                        Constraint::Fill(1),
                    ])
                    .split(instrs_layout[0])[1];

                ">".cyan().bold().render(target_pos, buf);
            }
        }

        let pc_pos = Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                Constraint::Length(self.registers.pc as u16),
                Constraint::Length(1),
                Constraint::Fill(1),
            ]);

        ">".green()
            .bold()
            .render(pc_pos.areas::<3>(instrs_layout[0])[1], buf);

        if let Some(InstructionUIState {
            text: Some(area), ..
        }) = self.state
        {
            let idx_width = Layout::horizontal([
                Constraint::Length(idx_width as u16 + 1),
                Constraint::Fill(1),
            ]);

            let row = pc_pos.areas::<3>(instrs_layout[1])[1];
            let text = idx_width.areas::<2>(row)[1];
            area.render(text, buf);
        }
    }
}

#[derive(Copy, Clone)]
struct InstructionExplanation<'a> {
    instr: Result<&'a Instruction, &'a str>,
    registers: &'a Registers,
    memory: &'a Memory,
}

impl<'a> Widget for InstructionExplanation<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = make_title("Explanation", false);

        let block = Block::bordered().title(title).border_set(border::ROUNDED);

        let text = match self.instr {
            Ok(instr) => Text::from(vec![
                Line::from(instr.explain_unsub()),
                Line::from(instr.explain_sub(self.registers, self.memory)),
            ]),
            Err(t) => Text::from(
                t.lines()
                    .map(|x| Line::from(x.red().bold()))
                    .collect::<Vec<_>>(),
            ),
        };

        Paragraph::new(text).block(block).render(area, buf)
    }
}

pub struct InstructionUIState {
    // no need for selected instruction -- this is just PC
    pub text: Option<TextArea<'static>>,
    pub prev_err: Option<Error>,
}

impl InstructionUIState {
    pub fn new() -> Self {
        Self {
            text: None,
            prev_err: None,
        }
    }

    /// returns column number of text area
    fn try_set_line(&mut self, state: &mut Simulator) -> usize {
        let area = self.text.take().unwrap();

        let instr = area.lines()[0].trim().parse::<Instruction>();

        if let Ok(instr) = instr {
            state.instructions[state.registers.pc as usize] = instr;
        } else {
            state.instructions[state.registers.pc as usize] =
                Instruction::Comment(area.lines()[0].clone());
        }

        area.cursor().1
    }

    fn make_line<'a>(
        &'a mut self,
        state: &Simulator,
        idx: Option<u64>,
    ) -> &'a mut TextArea<'static> {
        let instr = &state.instructions[state.registers.pc as usize];

        let text = format!("{}", instr);

        let len = text.len() as u16;

        let area = self.text.insert(TextArea::new(vec![text]));

        area.move_cursor(tui_textarea::CursorMove::Jump(
            0,
            idx.map(|x| x as u16).unwrap_or(len),
        ));

        area
    }

    pub fn handle(&mut self, input: Input, state: &mut Simulator) {
        if self.text.is_none() {
            match input {
                Input {
                    key: Key::Enter, ..
                } => self.prev_err = state.tick().err(),

                Input { key: Key::Up, .. } => {
                    state.registers.pc = state.registers.pc.saturating_sub(1);
                }
                Input { key: Key::Down, .. } => {
                    state.registers.pc = state
                        .instructions
                        .len()
                        .min(state.registers.pc as usize + 1)
                        as u64;
                }

                Input {
                    key: Key::Char('r'),
                    ctrl: true,
                    ..
                } => {
                    if state.registers.pc == state.instructions.len() as u64 {
                        state.instructions.push(Instruction::None);
                    }

                    let instr = &state.instructions[state.registers.pc as usize];

                    let str_repr = format!("{}", instr);

                    self.text = Some(TextArea::new(vec![str_repr]));
                }

                _ => {}
            }

            return;
        }

        // Now we deal with the much more complex case of the instruction editor.
        match input {
            Input { key: Key::Esc, .. } => {
                self.try_set_line(state);
            }

            Input { key: Key::Up, .. } => {
                if state.registers.pc == 0 {
                    return;
                }

                let cursor = self.try_set_line(state);

                state.registers.pc -= 1;

                self.make_line(state, Some(cursor as u64));
            }

            Input { key: Key::Down, .. } => {
                if state.registers.pc as usize == state.instructions.len() - 1 {
                    return;
                }

                let cursor = self.try_set_line(state);

                state.registers.pc += 1;

                self.make_line(state, Some(cursor as u64));
            }

            Input { key: Key::Left, .. } if self.text.as_ref().unwrap().cursor().1 == 0 => {
                if state.registers.pc == 0 {
                    return;
                }

                self.try_set_line(state);

                state.registers.pc -= 1;

                self.make_line(state, None);
            }

            Input {
                key: Key::Right, ..
            } if self
                .text
                .as_ref()
                .map(|x| x.cursor().1 == x.lines()[0].len())
                .unwrap() =>
            {
                if state.registers.pc == state.instructions.len() as u64 - 1 {
                    return;
                }

                self.try_set_line(state);

                state.registers.pc += 1;

                self.make_line(state, Some(0));
            }

            Input {
                key: Key::Enter, ..
            } => {
                let text = self.text.as_mut().unwrap();

                text.input(Input {
                    key: Key::Enter,
                    ..Default::default()
                });

                let first_line = &*text.lines()[0];
                let instr = first_line.parse::<Instruction>();
                let instr = if let Ok(i) = instr {
                    i
                } else {
                    Instruction::Comment(first_line.to_string())
                };

                state
                    .instructions
                    .insert(state.registers.pc as usize, instr);

                state.registers.pc += 1;

                state.instructions[state.registers.pc as usize] = Instruction::None;

                let new_line = text.lines()[1].clone();

                *text = TextArea::new(vec![new_line]);
            }

            Input {
                key: Key::Backspace,
                ..
            } if self.text.as_ref().unwrap().cursor().1 == 0 => {
                if state.registers.pc == 0 {
                    return;
                }

                let text = self.text.as_mut().unwrap();

                state.instructions.remove(state.registers.pc as usize);

                state.registers.pc -= 1;

                let prev_instr = format!("{}", &state.instructions[state.registers.pc as usize]);

                let new_text_line = format!("{}{}", prev_instr, &text.lines()[0]);

                *text = TextArea::new(vec![new_text_line]);
                text.move_cursor(tui_textarea::CursorMove::Jump(0, prev_instr.len() as u16));
            }

            input => {
                self.text.as_mut().unwrap().input(input);
            }
        }
    }
}
