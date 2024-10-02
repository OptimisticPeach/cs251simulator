use std::{collections::HashSet, ops::Range};

use ratatui::{
    layout::{Constraint, Flex, Layout, Rect},
    style::Stylize,
    text::Line,
    widgets::block::Title,
};

use crate::simulator::Memory;

pub fn get_ranges(
    memory: &Memory,
    around: u64,
    extras: impl IntoIterator<Item = u64>,
) -> Vec<Range<u64>> {
    let mut to_see = HashSet::new();

    for entry in memory.get_used().chain(extras) {
        to_see.insert(entry);

        for i in 1..around + 1 {
            let up = entry.saturating_add(i);
            let down = entry.saturating_sub(i);

            to_see.insert(up);
            to_see.insert(down);
        }
    }

    let mut to_see = to_see.into_iter().collect::<Vec<_>>();
    to_see.sort_unstable();

    let mut result = Vec::new();

    if to_see.is_empty() {
        return vec![];
    }

    let mut start = to_see[0];
    let mut prev = to_see[0];

    for i in 1..to_see.len() {
        let next = to_see[i];
        if next - prev != 1 {
            result.push(start..prev + 1);
            start = next;
        }

        prev = next;
    }

    result.push(start..prev + 1);

    result
}

pub fn center(area: Rect, horizontal: Constraint, vertical: Constraint) -> Rect {
    let [area] = Layout::horizontal([horizontal])
        .flex(Flex::Center)
        .areas(area);
    let [area] = Layout::vertical([vertical]).flex(Flex::Center).areas(area);
    area
}

pub fn make_title(name: &'static str, picked: bool) -> Title {
    if picked {
        Title::from(Line::default().spans([
            " ".into(),
            name.bold().blue().underlined(),
            " ".into(),
        ]))
    } else {
        Title::from(Line::default().spans([" ".into(), name.bold().blue(), " ".into()]))
    }
}
