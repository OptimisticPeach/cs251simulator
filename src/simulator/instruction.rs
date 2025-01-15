use color_eyre::{
    eyre::{bail, OptionExt},
    Result,
};
use pest::{iterators::Pair, Parser};
use ratatui::{style::Stylize, text::Span};
use serde::{Deserialize, Serialize};

use std::fmt::{Debug, Display};

use super::{Memory, Registers};

#[derive(pest_derive::Parser)]
#[grammar = "simulator/grammar.pest"]
struct InstructionParser;

fn parse_reg(x: Pair<Rule>) -> Result<u8> {
    assert_eq!(x.as_rule(), Rule::register);
    if x.as_span().as_str() == "XZR" {
        return Ok(31);
    }

    let num = x.into_inner().next().unwrap();
    assert_eq!(num.as_rule(), Rule::pos_number);

    let num = num.as_span().as_str().parse::<u8>()?;
    Ok(num)
}

fn parse_literal(x: Pair<Rule>) -> Result<i128> {
    assert_eq!(x.as_rule(), Rule::literal);

    let literal_num = x.into_inner().next().unwrap();
    assert_eq!(literal_num.as_rule(), Rule::literal_num);

    let num = literal_num.as_span().as_str().parse::<i128>()?;

    Ok(num)
}

fn parse_offset(x: Pair<Rule>) -> Result<Offset> {
    assert_eq!(x.as_rule(), Rule::offset);

    let mut iter = x.into_inner();

    let reg = parse_reg(iter.next().unwrap())?;
    let offset = parse_literal(iter.next().unwrap())?;

    Ok(Offset(reg, offset))
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Offset(pub u8, pub i128);

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Instruction {
    Add(u8, u8, u8),
    Sub(u8, u8, u8),
    AddI(u8, u8, i128),
    SubI(u8, u8, i128),
    Load(u8, Offset),
    Store(u8, Offset),
    Branch(i128),
    BranchZero(u8, i128),
    BranchNotZero(u8, i128),
    None,
    Comment(String),
}

impl Debug for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

impl Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Instruction::Add(r0, r1, r2) => {
                write!(f, "add  X{r0}, X{r1}, X{r2}")
            }
            Instruction::Sub(r0, r1, r2) => {
                write!(f, "sub  X{r0}, X{r1}, X{r2}")
            }
            Instruction::AddI(r0, r1, lit) => {
                write!(f, "addi X{r0}, X{r1}, #{lit}")
            }
            Instruction::SubI(r0, r1, lit) => {
                write!(f, "subi X{r0}, X{r1}, #{lit}")
            }
            Instruction::Load(r0, Offset(r1, lit)) => {
                write!(f, "ldur X{r0}, [X{r1}, #{lit}]")
            }
            Instruction::Store(r0, Offset(r1, lit)) => {
                write!(f, "stur X{r0}, [X{r1}, #{lit}]")
            }
            Instruction::Branch(lit) => {
                write!(f, "b    #{lit}")
            }
            Instruction::BranchZero(r0, lit) => {
                write!(f, "cbz  X{r0}, #{lit}")
            }
            Instruction::BranchNotZero(r0, lit) => {
                write!(f, "cbnz X{r0}, #{lit}")
            }
            Instruction::None => {
                write!(f, "")
            }
            Instruction::Comment(s) => {
                write!(f, "//{s}")
            }
        }
    }
}

fn make3<'a, T0, T1, T2, F, U>(
    mut iter: impl Iterator<Item = Pair<'a, Rule>>,
    f0: impl FnOnce(Pair<'a, Rule>) -> Result<T0>,
    f1: impl FnOnce(Pair<'a, Rule>) -> Result<T1>,
    f2: impl FnOnce(Pair<'a, Rule>) -> Result<T2>,
    result: F,
) -> Result<U>
where
    F: FnOnce(T0, T1, T2) -> U,
{
    let v0 = iter.next().ok_or_eyre("Insufficent arguments!")?;
    let v1 = iter.next().ok_or_eyre("Insufficent arguments!")?;
    let v2 = iter.next().ok_or_eyre("Insufficent arguments!")?;

    Ok(result(f0(v0)?, f1(v1)?, f2(v2)?))
}

fn make2<'a, T0, T1, F, U>(
    mut iter: impl Iterator<Item = Pair<'a, Rule>>,
    f0: impl FnOnce(Pair<'a, Rule>) -> Result<T0>,
    f1: impl FnOnce(Pair<'a, Rule>) -> Result<T1>,
    result: F,
) -> Result<U>
where
    F: FnOnce(T0, T1) -> U,
{
    let v0 = iter.next().ok_or_eyre("Insufficent arguments!")?;
    let v1 = iter.next().ok_or_eyre("Insufficent arguments!")?;

    Ok(result(f0(v0)?, f1(v1)?))
}

fn make1<'a, T0, F, U>(
    mut iter: impl Iterator<Item = Pair<'a, Rule>>,
    f0: impl FnOnce(Pair<'a, Rule>) -> Result<T0>,
    result: F,
) -> Result<U>
where
    F: FnOnce(T0) -> U,
{
    let v0 = iter.next().ok_or_eyre("Insufficent arguments!")?;

    Ok(result(f0(v0)?))
}

impl std::str::FromStr for Instruction {
    type Err = color_eyre::Report;

    fn from_str(mut s: &str) -> Result<Self, Self::Err> {
        s = s.trim();

        let s = s.to_uppercase();

        if s.len() == 0 {
            return Ok(Instruction::None);
        }

        let result = InstructionParser::parse(Rule::line, &s)?.next().unwrap();

        let full_line = result
            .into_inner()
            .next()
            .unwrap() // full_line
            .into_inner()
            .next()
            .unwrap() // comment | instruction
            .into_inner()
            .next()
            .unwrap() // specific instruction or comment_rest
        ;

        if full_line.as_rule() == Rule::comment_rest {
            return Ok(Instruction::Comment(full_line.as_span().as_str().into()));
        }

        let rule = full_line.as_rule();
        let iter = full_line.into_inner();

        let result = match rule {
            Rule::add => make3(iter, parse_reg, parse_reg, parse_reg, Instruction::Add),
            Rule::sub => make3(iter, parse_reg, parse_reg, parse_reg, Instruction::Sub),

            Rule::addi => make3(iter, parse_reg, parse_reg, parse_literal, Instruction::AddI),
            Rule::subi => make3(iter, parse_reg, parse_reg, parse_literal, Instruction::SubI),

            Rule::ldur => make2(iter, parse_reg, parse_offset, Instruction::Load),
            Rule::stur => make2(iter, parse_reg, parse_offset, Instruction::Store),

            Rule::branch => make1(iter, parse_literal, Instruction::Branch),
            Rule::cbz => make2(iter, parse_reg, parse_literal, Instruction::BranchZero),
            Rule::cbnz => make2(iter, parse_reg, parse_literal, Instruction::BranchNotZero),

            _ => panic!("{:?}", rule),
        };

        result.and_then(Instruction::validate)
    }
}

impl Instruction {
    pub fn validate(self) -> Result<Self> {
        use Instruction::*;

        match self {
            AddI(.., lit) | SubI(.., lit) => {
                if lit < 0 || lit >= 4096 {
                    bail!("Constant: #{lit} is too large!");
                }
            }
            Load(_, Offset(_, off)) | Store(_, Offset(_, off)) => {
                if off < -256 || off > 255 {
                    bail!("Offset #{off} is too large!");
                }
            }
            Branch(off) => {
                if off < -33554432 || off > 33554431 {
                    bail!("Jump #{off} is too large!");
                }
            }
            BranchZero(_, off) | BranchNotZero(_, off) => {
                if off < -262144 || off > 262143 {
                    bail!("Jump #{off} is too large!");
                }
            }
            _ => {}
        }

        Ok(self)
    }

    pub fn get_line(&self) -> Vec<Span> {
        use Instruction::*;

        let lines = match self {
            Add(x0, x1, x2) => vec![
                "add  ".blue(),
                format!("X{x0}").red(),
                ", ".into(),
                format!("X{x1}").red(),
                ", ".into(),
                format!("X{x2}").red(),
            ],
            Sub(x0, x1, x2) => vec![
                "sub  ".blue(),
                format!("X{x0}").red(),
                ", ".into(),
                format!("X{x1}").red(),
                ", ".into(),
                format!("X{x2}").red(),
            ],

            AddI(x0, x1, lit) => vec![
                "addi ".blue(),
                format!("X{x0}").red(),
                ", ".into(),
                format!("X{x1}").red(),
                ", ".into(),
                format!("#{lit}").yellow(),
            ],
            SubI(x0, x1, lit) => vec![
                "subi ".blue(),
                format!("X{x0}").red(),
                ", ".into(),
                format!("X{x1}").red(),
                ", ".into(),
                format!("#{lit}").yellow(),
            ],

            Load(x0, Offset(x1, off)) => vec![
                "ldur ".blue(),
                format!("X{x0}").red(),
                ", [".into(),
                format!("X{x1}").red(),
                ", ".into(),
                format!("#{off}").yellow(),
                "]".into(),
            ],

            Store(x0, Offset(x1, off)) => vec![
                "stur ".blue(),
                format!("X{x0}").red(),
                ", [".into(),
                format!("X{x1}").red(),
                ", ".into(),
                format!("#{off}").yellow(),
                "]".into(),
            ],

            Branch(off) => vec!["b    ".blue(), format!("#{off}").yellow()],
            BranchZero(x0, off) => vec![
                "cbz  ".blue(),
                format!("X{x0}").red(),
                ", ".into(),
                format!("#{off}").yellow(),
            ],
            BranchNotZero(x0, off) => vec![
                "cbnz ".blue(),
                format!("X{x0}").red(),
                ", ".into(),
                format!("#{off}").yellow(),
            ],
            None => vec![],
            Comment(s) => vec![
                "//".light_green().italic(),
                s.clone().light_green().italic(),
            ],
        };

        lines
    }

    pub fn explain_unsub(&self) -> Vec<Span> {
        use Instruction::*;

        match self {
            Add(x0, x1, x2) => vec![
                format!("X{x0}").red().bold(),
                " = ".into(),
                format!("X{x1}").red().bold(),
                " + ".into(),
                format!("X{x2}").red().bold(),
            ],
            Sub(x0, x1, x2) => vec![
                format!("X{x0}").red().bold(),
                " = ".into(),
                format!("X{x1}").red().bold(),
                " - ".into(),
                format!("X{x2}").red().bold(),
            ],

            AddI(x0, x1, lit) => vec![
                format!("X{x0}").red().bold(),
                " = ".into(),
                format!("X{x1}").red().bold(),
                " + ".into(),
                format!("{lit}").yellow(),
            ],
            SubI(x0, x1, lit) => vec![
                format!("X{x0}").red().bold(),
                " = ".into(),
                format!("X{x1}").red().bold(),
                " - ".into(),
                format!("{lit}").yellow(),
            ],

            Load(x0, Offset(x1, lit)) => vec![
                format!("X{x0}").red().bold(),
                " = ".into(),
                "M".light_magenta().bold(),
                "[".into(),
                format!("X{x1}").red().bold(),
                " + ".into(),
                format!("{lit}").yellow(),
                "]".into(),
            ],
            Store(x0, Offset(x1, lit)) => vec![
                "M".light_magenta().bold(),
                "[".into(),
                format!("X{x1}").red().bold(),
                " + ".into(),
                format!("{lit}").yellow(),
                "]".into(),
                " = ".into(),
                format!("X{x0}").red().bold(),
            ],

            Branch(lit) => vec![
                "PC".green().bold(),
                " = ".into(),
                "PC".green().bold(),
                " + ".into(),
                format!("{lit}").yellow(),
                " * 4".into(),
            ],
            BranchZero(x0, lit) => vec![
                "if ".into(),
                format!("X{x0}").red().bold(),
                " == 0: ".into(),
                "PC".green().bold(),
                " = ".into(),
                "PC".green().bold(),
                " + ".into(),
                format!("{lit}").yellow(),
                " * 4".into(),
            ],
            BranchNotZero(x0, lit) => vec![
                "if ".into(),
                format!("X{x0}").red().bold(),
                " != 0: ".into(),
                "PC".green().bold(),
                " = ".into(),
                "PC".green().bold(),
                " + ".into(),
                format!("{lit}").yellow(),
                " * 4".into(),
            ],

            None | Comment(_) => vec!["Stop Program".magenta().bold()],
        }
    }

    pub fn explain_sub(&self, registers: &Registers, memory: &Memory) -> Vec<Span> {
        use Instruction::*;

        match *self {
            Add(x0, x1, x2) => vec![
                format!("X{x0}").red().bold(),
                " = ".into(),
                format!("{}", registers.get(x1).unwrap()).red().bold(),
                " + ".into(),
                format!("{}", registers.get(x2).unwrap()).red().bold(),
                " = ".into(),
                format!(
                    "{}",
                    registers
                        .get(x1)
                        .unwrap()
                        .wrapping_add(registers.get(x2).unwrap())
                )
                .yellow(),
            ],
            Sub(x0, x1, x2) => vec![
                format!("X{x0}").red().bold(),
                " = ".into(),
                format!("{}", registers.get(x1).unwrap()).red().bold(),
                " - ".into(),
                format!("{}", registers.get(x2).unwrap()).red().bold(),
                " = ".into(),
                format!(
                    "{}",
                    registers
                        .get(x1)
                        .unwrap()
                        .wrapping_sub(registers.get(x2).unwrap())
                )
                .yellow(),
            ],

            AddI(x0, x1, lit) => vec![
                format!("X{x0}").red().bold(),
                " = ".into(),
                format!("{}", registers.get(x1).unwrap()).red().bold(),
                " + ".into(),
                format!("{lit}").yellow(),
                " = ".into(),
                format!("{}", registers.get(x1).unwrap() as i128 + lit).yellow(),
            ],
            SubI(x0, x1, lit) => vec![
                format!("X{x0}").red().bold(),
                " = ".into(),
                format!("{}", registers.get(x1).unwrap()).red().bold(),
                " - ".into(),
                format!("{lit}").yellow(),
                " = ".into(),
                format!("{}", registers.get(x1).unwrap() as i128 + lit).yellow(),
            ],

            Load(x0, Offset(x1, lit)) => {
                let addr = (registers.get(x1).unwrap() as i128 + lit) as u64;
                let valid = addr % 8 == 0;

                vec![
                    format!("X{x0}").red().bold(),
                    " = ".into(),
                    "M".light_magenta().bold(),
                    "[".into(),
                    format!("{}", registers.get(x1).unwrap()).red().bold(),
                    " + ".into(),
                    format!("{lit}").yellow(),
                    " = ".into(),
                    if valid {
                        format!("{}", addr).yellow()
                    } else {
                        format!("{}", addr).red().underlined().bold()
                    },
                    "]".into(),
                    " = ".into(),
                    memory
                        .get(addr)
                        .map(|x| format!("{x}").yellow())
                        .unwrap_or("ERROR".red().underlined().bold().slow_blink()),
                ]
            }
            Store(x0, Offset(x1, lit)) => {
                let addr = (registers.get(x1).unwrap() as i128 + lit) as u64;
                let valid = addr % 8 == 0;

                vec![
                    "M".light_magenta().bold(),
                    "[".into(),
                    format!("{}", registers.get(x1).unwrap()).red().bold(),
                    " + ".into(),
                    format!("{lit}").yellow(),
                    " = ".into(),
                    if valid {
                        format!("{}", addr).yellow()
                    } else {
                        format!("{}", addr).red().underlined().bold()
                    },
                    "]".into(),
                    " = ".into(),
                    format!("{}", registers.get(x0).unwrap()).red().bold(),
                ]
            }

            Branch(lit) => vec![
                "PC".green().bold(),
                " = ".into(),
                format!("{}", registers.pc * 4).green().bold(),
                " + ".into(),
                format!("{lit}").yellow(),
                " * 4 = ".into(),
                format!("{}", (registers.pc as i128 + lit) * 4).yellow(),
            ],
            BranchZero(x0, lit) => vec![
                "if ".into(),
                format!("{}", registers.get(x0).unwrap()).red().bold(),
                " == 0: ".into(),
                "PC".green().bold(),
                " = ".into(),
                format!("{}", registers.pc * 4).green().bold(),
                " + ".into(),
                format!("{lit}").yellow(),
                " * 4 = ".into(),
                format!("{}", (registers.pc as i128 + lit) * 4).yellow(),
            ],
            BranchNotZero(x0, lit) => vec![
                "if ".into(),
                format!("{}", registers.get(x0).unwrap()).red().bold(),
                " != 0: ".into(),
                "PC".green().bold(),
                " = ".into(),
                format!("{}", registers.pc * 4).green().bold(),
                " + ".into(),
                format!("{lit}").yellow(),
                " * 4 = ".into(),
                format!("{}", (registers.pc as i128 + lit) * 4).yellow(),
            ],

            None | Comment(_) => vec!["Stop Program".magenta().bold()],
        }
    }

    pub fn is_reg_highlighted(&self, register: u8) -> Option<Highlight> {
        use Instruction::*;

        // Instruction has a `None` variant which overshadows
        // Option's None. We use options a lot more frequently
        // in this code, so we reintroduce Option's None.
        use Option::None;

        match *self {
            Add(x0, x1, x2) | Sub(x0, x1, x2) => {
                if register == x0 {
                    Some(Highlight::Dest)
                } else if register == x1 || register == x2 {
                    Some(Highlight::Source)
                } else {
                    None
                }
            }
            AddI(x0, x1, _) | SubI(x0, x1, _) => {
                if register == x0 {
                    Some(Highlight::Dest)
                } else if register == x1 {
                    Some(Highlight::Source)
                } else {
                    None
                }
            }

            Load(x0, Offset(x1, _)) => {
                if register == x0 {
                    Some(Highlight::Dest)
                } else if register == x1 {
                    Some(Highlight::Source)
                } else {
                    None
                }
            }
            Store(x0, Offset(x1, _)) => {
                if register == x0 {
                    Some(Highlight::Source)
                } else if register == x1 {
                    Some(Highlight::Source)
                } else {
                    None
                }
            }

            BranchZero(x0, _) | BranchNotZero(x0, _) => {
                if register == x0 {
                    Some(Highlight::Source)
                } else {
                    None
                }
            }

            Branch(_) | Instruction::None | Comment(_) => None,
        }
    }

    pub fn highlighted_mem(&self, registers: &Registers) -> Option<(u64, Highlight)> {
        match *self {
            Instruction::Load(_, Offset(x0, off)) | Instruction::Store(_, Offset(x0, off)) => {
                let value = registers.get(x0).unwrap();
                let addr = (value as i128 + off) as u64;

                if addr % 8 != 0 {
                    None
                } else {
                    let addr = addr / 8;
                    match *self {
                        Instruction::Load(..) => Some((addr, Highlight::Source)),
                        Instruction::Store(..) => Some((addr, Highlight::Dest)),
                        _ => unreachable!(),
                    }
                }
            }
            _ => None,
        }
    }

    pub fn highlighted_instr(&self, pc: u64) -> Option<u64> {
        if let Instruction::Branch(off)
        | Instruction::BranchZero(_, off)
        | Instruction::BranchNotZero(_, off) = self
        {
            Some((pc as i128 + off) as u64)
        } else {
            None
        }
    }
}

#[derive(Clone, Copy)]
pub enum Highlight {
    Source,
    Dest,
}
