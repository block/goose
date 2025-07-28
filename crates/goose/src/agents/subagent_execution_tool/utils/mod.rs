use std::collections::HashMap;

use crate::agents::subagent_execution_tool::task_types::{TaskInfo, TaskStatus};

pub fn get_task_name(task_info: &TaskInfo) -> &str {
    task_info
        .task
        .get_sub_recipe_name()
        .unwrap_or(&task_info.task.id)
}

pub fn count_by_status(tasks: &HashMap<String, TaskInfo>) -> (usize, usize, usize, usize, usize) {
    let total = tasks.len();
    let (pending, running, completed, failed) = tasks.values().fold(
        (0, 0, 0, 0),
        |(pending, running, completed, failed), task| match task.status {
            TaskStatus::Pending => (pending + 1, running, completed, failed),
            TaskStatus::Running => (pending, running + 1, completed, failed),
            TaskStatus::Completed => (pending, running, completed + 1, failed),
            TaskStatus::Failed => (pending, running, completed, failed + 1),
        },
    );
    (total, pending, running, completed, failed)
}

pub fn strip_ansi_codes(text: &str) -> String {
    let mut result = String::new();
    let mut chars = text.chars();

    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            if let Some(next_ch) = chars.next() {
                if next_ch == '[' {
                    // This is an ANSI escape sequence, consume until alphabetic character
                    loop {
                        match chars.next() {
                            Some(c) if c.is_ascii_alphabetic() => break,
                            Some(_) => continue,
                            None => break,
                        }
                    }
                } else {
                    // Not an ANSI sequence, keep both characters
                    result.push(ch);
                    result.push(next_ch);
                }
            } else {
                // End of string after \x1b
                result.push(ch);
            }
        } else {
            result.push(ch);
        }
    }

    result
}

#[cfg(test)]
mod tests;
