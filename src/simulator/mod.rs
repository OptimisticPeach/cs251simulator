mod registers;
use color_eyre::eyre::Result;
use instruction::Offset;
pub use registers::Registers;

mod memory;
pub use memory::Memory;

mod instruction;
pub use instruction::{Highlight, Instruction};
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Simulator {
    pub registers: Registers,
    pub memory: Memory,
    pub instructions: Vec<Instruction>,
}

impl Simulator {
    pub fn new() -> Self {
        Self {
            registers: Registers::new(),
            memory: Memory::new(),
            instructions: Vec::new(),
        }
    }

    pub fn tick(&mut self) -> Result<RunningState> {
        let pc = self.registers.pc as usize;

        if pc >= self.instructions.len() {
            return Ok(RunningState::ShouldStop);
        }

        let instr = &self.instructions[pc];

        let mut pc_diff = 1;

        match *instr {
            Instruction::Add(r0, r1, r2) => {
                let vr1 = self.registers.get(r1)?;
                let vr2 = self.registers.get(r2)?;

                let (result, _) = vr1.overflowing_add(vr2);

                self.registers.set(r0, result)?;
            }

            Instruction::Sub(r0, r1, r2) => {
                let vr1 = self.registers.get(r1)?;
                let vr2 = self.registers.get(r2)?;

                let (result, _) = vr1.overflowing_sub(vr2);

                self.registers.set(r0, result)?;
            }

            Instruction::AddI(r0, r1, lit) => {
                let vr1 = self.registers.get(r1)?;

                let (result, _) = vr1.overflowing_add(lit as u64);

                self.registers.set(r0, result)?;
            }

            Instruction::SubI(r0, r1, lit) => {
                let vr1 = self.registers.get(r1)?;

                let (result, _) = vr1.overflowing_sub(lit as u64);

                self.registers.set(r0, result)?;
            }

            Instruction::Load(r0, Offset(r1, off)) => {
                let addr = self.registers.get(r1)?;
                let new_addr = addr as i128 + off;
                let truncated = new_addr & (u64::MAX as i128);
                let truncated = truncated as u64;

                let val = self.memory.get(truncated)?;

                self.registers.set(r0, val)?;
            }

            Instruction::Store(r0, Offset(r1, off)) => {
                let addr = self.registers.get(r1)?;
                let new_addr = addr as i128 + off;
                let truncated = new_addr & (u64::MAX as i128);
                let truncated = truncated as u64;

                let val = self.registers.get(r0)?;

                self.memory.set(truncated, val)?;
            }

            Instruction::Branch(off) => {
                pc_diff = off;
            }

            Instruction::BranchZero(r0, off) => {
                let val = self.registers.get(r0)?;

                if val == 0 {
                    pc_diff = off;
                }
            }

            Instruction::BranchNotZero(r0, off) => {
                let val = self.registers.get(r0)?;

                if val != 0 {
                    pc_diff = off;
                }
            }

            Instruction::None | Instruction::Comment(_) => return Ok(RunningState::ShouldStop),
        }

        let new_pc = self.registers.pc as i128 + pc_diff;

        let new_pc = (new_pc & u64::MAX as i128) as u64;

        self.registers.pc = new_pc;

        Ok(RunningState::KeepRunning)
    }
}

pub enum RunningState {
    KeepRunning,
    ShouldStop,
}
