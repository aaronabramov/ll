use super::Level;
use crate::TaskInternal;

pub fn parse_level(task_internal: &TaskInternal) -> Level {
    let mut all_level_tags = vec![];
    for tag in &task_internal.tags {
        match tag.as_str() {
            "l0" => all_level_tags.push(Level::L0),
            "l1" => all_level_tags.push(Level::L1),
            "l2" => all_level_tags.push(Level::L2),
            "l3" => all_level_tags.push(Level::L3),
            _ => (),
        }
    }

    all_level_tags.into_iter().min().unwrap_or(Level::L1)
}
