use color_eyre::eyre::Result;
use ratatui::{
    crossterm::event::{self, Event, KeyEvent, KeyEventKind},
    layout::{Constraint, Layout},
    style::Stylize,
    text::{Line, Span},
    DefaultTerminal, Frame,
};
use tui_textarea::{Input, Key};

use crate::simulator::Simulator;

mod registers;
use registers::{RegisterUI, RegisterUIState};

mod memory;
use memory::{MemoryUI, MemoryUIState, PersistentMemoryState};

mod instruction;
use instruction::{InstructionUI, InstructionUIState};

mod picker;
use picker::Picker;

mod io;
use io::{LoadFocus, LoadUIState, SaveUIState};

enum Focus {
    Memory(MemoryUIState),
    Registers(RegisterUIState),
    Instructions(InstructionUIState),
    Save(SaveUIState),
    Load(LoadUIState),
}

pub struct Tui {
    focus: Focus,
    running: bool,
    picking: bool,
    state: Simulator,

    persistent_memory: PersistentMemoryState,
}

impl Tui {
    pub fn new(state: Simulator) -> Self {
        Self {
            running: true,
            picking: false,
            focus: Focus::Instructions(InstructionUIState::new()),
            state,

            persistent_memory: PersistentMemoryState::new(),
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        terminal.draw(|frame| self.draw(frame))?;
        while self.running {
            self.handle_events()?;
            terminal.draw(|frame| self.draw(frame))?;
        }

        Ok(())
    }

    fn handle_events(&mut self) -> Result<()> {
        let event = event::read()?;
        if let Event::Key(KeyEvent {
            kind: KeyEventKind::Release,
            ..
        }) = event
        {
            return Ok(());
        }

        match event.into() {
            Input {
                key: Key::Char('q'),
                ctrl: true,
                ..
            } => self.running = false,

            Input {
                key: Key::Char('w'),
                ctrl: true,
                ..
            } => self.picking = true,

            event if self.picking => {
                match event {
                    Input {
                        key: Key::Char('i'),
                        ..
                    } => self.focus = Focus::Instructions(InstructionUIState::new()),
                    Input {
                        key: Key::Char('m'),
                        ..
                    } => {
                        self.focus =
                            Focus::Memory(MemoryUIState::new(self.persistent_memory.selected.get()))
                    }
                    Input {
                        key: Key::Char('r'),
                        ..
                    } => self.focus = Focus::Registers(RegisterUIState::new()),
                    Input {
                        key: Key::Char('s'),
                        ..
                    } => self.focus = Focus::Save(SaveUIState::new()),
                    Input {
                        key: Key::Char('l'),
                        ..
                    } => self.focus = Focus::Load(LoadUIState::new()),
                    _ => {}
                }

                self.picking = false;
            }

            event => match &mut self.focus {
                Focus::Instructions(state) => state.handle(event, &mut self.state),
                Focus::Registers(state) => state.handle(event, &mut self.state),
                Focus::Memory(state) => state.handle(event, &mut self.state.memory),
                Focus::Save(state) => {
                    if state.handle(event, &self.state) {
                        self.focus = Focus::Instructions(InstructionUIState::new());
                    }
                }
                Focus::Load(state) => {
                    if state.handle(event, &mut self.state) {
                        self.focus = Focus::Instructions(InstructionUIState::new());

                        self.persistent_memory = PersistentMemoryState::new();
                    }
                }
            },
        }
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        let command_list_layout = Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([Constraint::Fill(1), Constraint::Length(1)])
            .split(frame.area());

        let main_layout = Layout::default()
            .direction(ratatui::layout::Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(command_list_layout[0]);

        let layout_reg_mem = Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([Constraint::Length(18), Constraint::Fill(1)])
            .split(main_layout[1]);

        let registers = RegisterUI {
            registers: &self.state.registers,
            instrs: &self.state.instructions,
            state: if let Focus::Registers(reg) = &self.focus {
                Some(reg)
            } else {
                None
            },
        };

        frame.render_widget(registers, layout_reg_mem[0]);

        let memory = MemoryUI {
            memory: &self.state.memory,
            registers: &self.state.registers,
            instrs: &self.state.instructions,
            state: if let Focus::Memory(state) = &self.focus {
                Some(state)
            } else {
                None
            },

            persistent: &self.persistent_memory,
        };

        frame.render_widget(memory, layout_reg_mem[1]);

        let instructions = InstructionUI {
            instrs: &self.state.instructions,
            registers: &self.state.registers,
            memory: &self.state.memory,
            pc: self.state.registers.pc,
            state: if let Focus::Instructions(state) = &self.focus {
                Some(state)
            } else {
                None
            },
        };

        frame.render_widget(instructions, main_layout[0]);

        if self.picking {
            let mut picker = Picker::new('r');
            frame.render_widget(picker, layout_reg_mem[0]);

            picker = Picker::new('m');
            frame.render_widget(picker, layout_reg_mem[1]);

            picker = Picker::new('i');
            frame.render_widget(picker, main_layout[0]);
        }

        let mut commands = self.get_commands();

        let mut prev = commands.next();

        let mut command_components = Vec::<Span<'static>>::new();

        while let Some(next) = commands.next() {
            let Some((key, expl)) = prev else {
                unreachable!();
            };

            command_components.push(expl.into());
            command_components.push(" ".into());
            command_components.push(key.light_blue().bold());
            command_components.push(" | ".into());

            prev = Some(next);
        }

        if let Some((key, expl)) = prev {
            command_components.push(expl.into());
            command_components.push(" ".into());
            command_components.push(key.light_blue().bold());
        }

        let explanations = Line::from(command_components);
        frame.render_widget(explanations, command_list_layout[1]);

        if let Focus::Save(state) = &self.focus {
            frame.render_widget(state, frame.area());
        } else if let Focus::Load(state) = &self.focus {
            frame.render_widget(state, frame.area());
        }
    }

    fn get_commands(&self) -> impl Iterator<Item = (&'static str, &'static str)> + 'static {
        let default = if self.picking {
            [("<Ctrl> <Q>", "Quit"), ("<L>", "Load"), ("<S>", "Save")][..].into_iter()
        } else {
            [("<Ctrl> <Q>", "Quit"), ("<Ctrl> <W>", "Window")][..].into_iter()
        };

        let window = match &self.focus {
            Focus::Instructions(state) => {
                if state.text.is_some() {
                    [("<Esc>", "Exit Edit Mode"), ("<any key>", "Edit")][..].into_iter()
                } else {
                    [
                        ("<Enter>", "Run 1"),
                        ("<Up>", "PC -= 4"),
                        ("<Down>", "PC += 4"),
                        ("<Ctrl> <R>", "Enter Edit Mode"),
                    ][..]
                        .into_iter()
                }
            }
            Focus::Registers(RegisterUIState { replacing, .. }) => match replacing {
                Some(_) => [("<Esc>", "Cancel"), ("<Enter>", "Accept")][..].into_iter(),
                None => [("<Arrow Key>", "Pick"), ("<Ctrl> <R>", "Edit")][..].into_iter(),
            },
            Focus::Memory(MemoryUIState {
                insertion,
                line_selection,
                ..
            }) => {
                if insertion.is_some() || line_selection.is_some() {
                    [("<Esc>", "Cancel"), ("<Enter>", "Accept")][..].into_iter()
                } else {
                    [
                        ("<G>", "Goto Addr"),
                        ("<Ctrl> <R>", "Replace"),
                        ("<Arrow Up/Down>", "Navigate"),
                    ][..]
                        .into_iter()
                }
            }

            Focus::Load(LoadUIState { message, focus, .. }) => {
                if message.is_some() {
                    [("<Esc>", "Dismiss")][..].into_iter()
                } else {
                    match focus {
                        LoadFocus::Reg | LoadFocus::Mem | LoadFocus::Instr => [
                            ("<Esc>", "Cancel"),
                            ("<Enter>", "Toggle"),
                            ("<arrow key>", "Select"),
                        ][..]
                            .into_iter(),
                        LoadFocus::File => [
                            ("<Esc>", "Cancel"),
                            ("<Enter>", "Accept"),
                            ("<Up/Down>", "Select"),
                        ][..]
                            .into_iter(),
                    }
                }
            }

            Focus::Save(SaveUIState { message, .. }) => match message {
                Some(_) => [("<Esc>", "Dismiss")][..].into_iter(),

                None => [("<Esc>", "Cancel"), ("<Enter>", "Accept")][..].into_iter(),
            },
        };

        default.copied().chain(window.copied())
    }
}

pub fn setup_and_run_tui(simulator: Simulator) -> Result<()> {
    let mut terminal = ratatui::init();

    Tui::new(simulator).run(&mut terminal)?;

    ratatui::restore();

    Ok(())
}
