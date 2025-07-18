use std::collections::HashMap;

use crate::agents::sub_recipe_execution_tool::task_types::{TaskInfo, TaskStatus};

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

#[cfg(test)]
mod tests;
