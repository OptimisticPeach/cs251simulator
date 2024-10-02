use std::collections::HashMap;

use color_eyre::eyre::{eyre, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Memory {
    memory: HashMap<u64, u64>,
}

impl Memory {
    pub fn new() -> Self {
        Self {
            memory: HashMap::new(),
        }
    }

    pub fn get(&self, byte_addr: u64) -> Result<u64> {
        if byte_addr % 8 != 0 {
            Err(eyre!("Byte address {byte_addr} is not a multiple of 8!"))?;
        }

        let idx = byte_addr / 8;

        let val = self.memory.get(&idx).copied().unwrap_or(0);

        Ok(val)
    }

    pub fn set(&mut self, byte_addr: u64, val: u64) -> Result<()> {
        if byte_addr % 8 != 0 {
            Err(eyre!("Byte address {byte_addr} is not a mutiple of 8!"))?;
        }

        let idx = byte_addr / 8;

        if val == 0 {
            self.memory.remove(&idx);
        } else {
            self.memory.insert(idx, val);
        }

        Ok(())
    }

    /// returns slots, not memory addresses
    pub fn get_used<'a>(&'a self) -> impl Iterator<Item = u64> + 'a {
        self.memory.keys().copied()
    }
}
