/**
 * Complete Command - Mark a task as completed
 */

import { ArgumentsCamelCase } from 'yargs';
import { CompleteCommandArgs } from '../types';
import { TaskRepository } from '../db/repository';
import { validateId } from '../utils/validation';
import { formatSuccess, formatError, formatTask, formatWarning } from '../utils/formatter';

export const command = 'complete <id>';
export const describe = 'Mark a task as completed';
export const builder = {
  id: {
    describe: 'Task ID to mark as completed',
    type: 'number',
    demandOption: true,
  },
};

export async function handler(argv: ArgumentsCamelCase<CompleteCommandArgs>): Promise<void> {
  // 1. Validate ID using validateId
  const idValidation = validateId(argv.id);

  // 2. If invalid, print error and exit with code 1
  if (!idValidation.valid) {
    console.log(formatError(idValidation.error ?? 'Invalid task ID'));
    process.exit(1);
  }

  const taskId = idValidation.normalizedValue!;
  const repo = new TaskRepository();

  try {
    // 3. Check if task exists using findById
    const task = repo.findById(taskId);

    // 4. If not found, print "Task with ID X not found" and exit with code 1
    if (!task) {
      console.log(formatError(`Task with ID ${taskId} not found`));
      process.exit(1);
    }

    // 5. If already completed, print warning and exit
    if (task.completed) {
      console.log(formatWarning('Task is already completed'));
      process.exit(0);
    }

    // 6. Update task: completed=true (repository will set completedAt automatically)
    const updatedTask = repo.update(taskId, { completed: true });

    if (!updatedTask) {
      console.log(formatError('Failed to update task'));
      process.exit(1);
    }

    // 7. Print success message with the completed task
    console.log(formatSuccess('Task completed!'));
    console.log(formatTask(updatedTask));
  } catch (error) {
    const message = error instanceof Error ? error.message : 'Unknown error';
    console.log(formatError(message));
    process.exit(1);
  } finally {
    repo.close();
  }
}
