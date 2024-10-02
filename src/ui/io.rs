use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Styled as _, Stylize as _},
    symbols::border,
    text::{Line, Span},
    widgets::{Block, Clear, Widget},
};
use tui_textarea::{Input, Key, TextArea};

use serde_json::{from_str, to_string_pretty};

use crate::{
    simulator::Simulator,
    util::{center, make_title},
};

pub struct SaveUIState {
    pub area: TextArea<'static>,
    pub message: Option<String>,
}

impl Widget for &SaveUIState {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = make_title("Save To File", true);

        let area = center(area, Constraint::Percentage(75), Constraint::Length(3));

        let block = Block::bordered()
            .border_set(border::ROUNDED)
            .set_style(Style::reset().fg(Color::Cyan))
            .title(title);

        let inner = block.inner(area);

        block.render(area, buf);

        Clear.render(inner, buf);

        if let Some(x) = &self.message {
            x.clone().red().render(inner, buf);
        } else {
            let message = "File: ";
            let areas =
                Layout::horizontal([Constraint::Length(6), Constraint::Fill(1)]).areas::<2>(inner);

            message.render(areas[0], buf);

            self.area.render(areas[1], buf);
        }
    }
}

impl SaveUIState {
    pub fn new() -> Self {
        Self {
            area: TextArea::default(),
            message: None,
        }
    }

    pub fn handle(&mut self, event: Input, simulator: &Simulator) -> bool {
        if event.key == Key::Esc {
            return true;
        }

        if self.message.is_some() {
            return false;
        }

        if event.key == Key::Enter {
            let path = &self.area.lines()[0];

            let to_write = to_string_pretty(simulator);

            let to_write = match to_write {
                Ok(x) => x,
                Err(e) => {
                    self.message = Some(format!("{}", e));
                    return false;
                }
            };

            let err = std::fs::write(path, to_write);

            if let Err(e) = err {
                self.message = Some(format!("{}", e));
                return false;
            } else {
                return true;
            }
        }

        self.area.input(event);

        false
    }
}

pub struct LoadUIState {
    pub load_reg: bool,
    pub load_mem: bool,
    pub load_instr: bool,
    pub area: TextArea<'static>,
    pub message: Option<String>,
    pub focus: LoadFocus,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LoadFocus {
    Reg,
    Mem,
    Instr,
    File,
}

impl Widget for &LoadUIState {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = make_title("Load From File", true);

        let area = center(area, Constraint::Percentage(75), Constraint::Length(4));

        let block = Block::bordered()
            .border_set(border::ROUNDED)
            .set_style(Style::reset().fg(Color::Cyan))
            .title(title);

        let inner = block.inner(area);

        block.render(area, buf);

        Clear.render(inner, buf);

        let rows =
            Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas::<2>(inner);

        if let Some(x) = &self.message {
            x.clone().red().render(rows[0], buf);
            return;
        }

        let mut toggle_line = Vec::new();
        make_toggle(
            "Registers: ",
            self.focus == LoadFocus::Reg,
            self.load_reg,
            &mut toggle_line,
        );

        toggle_line.push("    ".into());

        make_toggle(
            "Memory: ",
            self.focus == LoadFocus::Mem,
            self.load_mem,
            &mut toggle_line,
        );

        toggle_line.push("    ".into());

        make_toggle(
            "Instructions: ",
            self.focus == LoadFocus::Instr,
            self.load_instr,
            &mut toggle_line,
        );

        Line::from(toggle_line).render(rows[0], buf);

        let message = "File: ";
        let areas =
            Layout::horizontal([Constraint::Length(6), Constraint::Fill(1)]).areas::<2>(rows[1]);

        if self.focus == LoadFocus::File {
            message.underlined().render(areas[0], buf);

            self.area.render(areas[1], buf);
        } else {
            message.render(areas[0], buf);

            self.area.lines()[0].clone().render(areas[1], buf);
        }
    }
}

impl LoadUIState {
    pub fn new() -> Self {
        Self {
            load_reg: true,
            load_mem: true,
            load_instr: true,
            area: TextArea::default(),
            message: None,
            focus: LoadFocus::File,
        }
    }

    pub fn handle(&mut self, event: Input, state: &mut Simulator) -> bool {
        if event.key == Key::Esc {
            return true;
        }

        if self.message.is_some() {
            return false;
        }

        if event.key == Key::Enter {
            match self.focus {
                LoadFocus::Reg => self.load_reg = !self.load_reg,
                LoadFocus::Mem => self.load_mem = !self.load_mem,
                LoadFocus::Instr => self.load_instr = !self.load_instr,

                LoadFocus::File => {
                    let loaded = std::fs::read_to_string(&self.area.lines()[0]);

                    let loaded = match loaded {
                        Ok(x) => x,
                        Err(e) => {
                            self.message = Some(format!("{}", e));
                            return false;
                        }
                    };

                    let deserialized = from_str::<Simulator>(&loaded);

                    let deserialized = match deserialized {
                        Ok(x) => x,
                        Err(e) => {
                            self.message = Some(format!("{}", e));
                            return false;
                        }
                    };

                    let Simulator {
                        registers,
                        memory,
                        instructions,
                    } = deserialized;

                    if self.load_reg {
                        state.registers = registers;
                    }

                    if self.load_mem {
                        state.memory = memory;
                    }

                    if self.load_instr {
                        state.instructions = instructions;
                    }

                    return true;
                }
            }
        }

        match event {
            Input {
                key: Key::Up | Key::Down,
                ..
            } => {
                self.focus = match self.focus {
                    LoadFocus::File => LoadFocus::Reg,
                    LoadFocus::Reg | LoadFocus::Mem | LoadFocus::Instr => LoadFocus::File,
                };
            }

            Input { key: Key::Left, .. } if self.focus != LoadFocus::File => {
                self.focus = match self.focus {
                    LoadFocus::Reg => LoadFocus::Instr,
                    LoadFocus::Mem => LoadFocus::Reg,
                    LoadFocus::Instr => LoadFocus::Mem,
                    LoadFocus::File => unreachable!(),
                };
            }

            Input {
                key: Key::Right, ..
            } if self.focus != LoadFocus::File => {
                self.focus = match self.focus {
                    LoadFocus::Reg => LoadFocus::Mem,
                    LoadFocus::Mem => LoadFocus::Instr,
                    LoadFocus::Instr => LoadFocus::Reg,
                    LoadFocus::File => unreachable!(),
                };
            }

            input if self.focus == LoadFocus::File => {
                self.area.input(input);
            }

            _ => {}
        }

        false
    }
}

fn make_toggle(name: &'static str, selected: bool, toggled: bool, into: &mut Vec<Span<'static>>) {
    if selected {
        into.push(name.underlined());
    } else {
        into.push(name.into());
    }

    let boxed = if toggled { "[X]" } else { "[ ]" };

    if selected {
        into.push(boxed.underlined());
    } else {
        into.push(boxed.into());
    }
}
