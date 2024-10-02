use color_eyre::{eyre::eyre, Result};
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Registers {
    registers: [u64; 31],
    pub pc: u64,
}

impl Registers {
    pub fn new() -> Self {
        Self {
            registers: [0; 31],
            pc: 0,
        }
    }

    pub fn get(&self, idx: u8) -> Result<u64> {
        match idx {
            0..31 => Ok(self.registers[idx as usize]),
            31 => Ok(0),
            _ => Err(eyre!("Register {idx} does not exist!")),
        }
    }

    pub fn set(&mut self, idx: u8, val: u64) -> Result<()> {
        match idx {
            0..31 => self.registers[idx as usize] = val,
            31 => {}
            _ => Err(eyre!("Register {idx} does not exist!"))?,
        }

        Ok(())
    }
}
