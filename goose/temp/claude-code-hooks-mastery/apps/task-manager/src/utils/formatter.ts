/**
 * Chalk-based output formatter for the Task Manager CLI
 * Provides consistent, colorful formatting for all CLI output
 */

import chalk from 'chalk';
import { Task, TaskStats, Priority } from '../types.js';

// =============================================================================
// Color Scheme Constants
// =============================================================================

const PRIORITY_COLORS = {
  [Priority.HIGH]: chalk.red,
  [Priority.MEDIUM]: chalk.yellow,
  [Priority.LOW]: chalk.green,
};

const PRIORITY_ICONS = {
  [Priority.HIGH]: '🔴',
  [Priority.MEDIUM]: '🟡',
  [Priority.LOW]: '🟢',
};

// =============================================================================
// Helper Functions
// =============================================================================

/**
 * Check if a task is overdue (past due date and not completed)
 */
export function isOverdue(task: Task): boolean {
  if (!task.dueDate || task.completed) {
    return false;
  }
  const today = new Date();
  today.setHours(0, 0, 0, 0);
  const dueDate = new Date(task.dueDate);
  return dueDate < today;
}

/**
 * Pad a string to a specific width
 */
function padRight(str: string, width: number): string {
  const visibleLength = str.replace(/\x1B\[[0-9;]*m/g, '').length;
  const padding = Math.max(0, width - visibleLength);
  return str + ' '.repeat(padding);
}

/**
 * Pad a string to a specific width (left padding)
 */
function padLeft(str: string, width: number): string {
  const visibleLength = str.replace(/\x1B\[[0-9;]*m/g, '').length;
  const padding = Math.max(0, width - visibleLength);
  return ' '.repeat(padding) + str;
}

/**
 * Create a progress bar
 */
function createProgressBar(percentage: number, width: number = 10): string {
  const filled = Math.round((percentage / 100) * width);
  const empty = width - filled;
  return chalk.green('█'.repeat(filled)) + chalk.gray('░'.repeat(empty));
}

// =============================================================================
// Task Formatting Functions
// =============================================================================

/**
 * Format a single task for display
 * Shows: [ID] Title (Priority) - Due: Date
 * Completed tasks are dimmed, overdue tasks are highlighted
 */
export function formatTask(task: Task): string {
  const idStr = chalk.gray(`#${task.id}`);
  const priorityColor = PRIORITY_COLORS[task.priority];
  const priorityStr = priorityColor(task.priority.toUpperCase());
  const dueDateStr = task.dueDate ? task.dueDate : '-';

  let titleStr: string;
  let statusStr: string;

  if (task.completed) {
    titleStr = chalk.dim.strikethrough(task.title);
    statusStr = chalk.green('✅ Done');
  } else if (isOverdue(task)) {
    titleStr = chalk.red(task.title);
    statusStr = chalk.red.bold('⚠️ Overdue');
  } else {
    titleStr = task.title;
    statusStr = chalk.blue('⏳ Pending');
  }

  return `${idStr}  ${titleStr} (${priorityStr}) - Due: ${dueDateStr} ${statusStr}`;
}

/**
 * Format task list with aligned columns
 * Columns: ID | Title | Priority | Due Date | Status
 */
export function formatTaskList(tasks: Task[]): string {
  if (tasks.length === 0) {
    return chalk.yellow('No tasks found.');
  }

  // Column widths
  const idWidth = 4;
  const titleWidth = 25;
  const priorityWidth = 8;
  const dateWidth = 12;
  const statusWidth = 12;

  // Header
  const headerLine = [
    padRight(chalk.bold('ID'), idWidth),
    padRight(chalk.bold('Title'), titleWidth),
    padRight(chalk.bold('Priority'), priorityWidth),
    padRight(chalk.bold('Due Date'), dateWidth),
    chalk.bold('Status'),
  ].join(' ');

  const separator = chalk.gray('─'.repeat(idWidth + titleWidth + priorityWidth + dateWidth + statusWidth + 4));

  // Format each task row
  const rows = tasks.map((task) => {
    const idStr = chalk.gray(`#${task.id}`);
    const priorityColor = PRIORITY_COLORS[task.priority];
    const priorityStr = priorityColor(task.priority.toUpperCase());
    const dueDateStr = task.dueDate || '-';

    let titleStr: string;
    let statusStr: string;
    let prefix = '  ';

    if (task.completed) {
      titleStr = chalk.dim.strikethrough(task.title.slice(0, titleWidth - 2));
      statusStr = chalk.green('✅ Done');
      prefix = chalk.green('✓ ');
    } else if (isOverdue(task)) {
      titleStr = chalk.red(task.title.slice(0, titleWidth - 2));
      statusStr = chalk.red.bold('⚠️ Overdue');
      prefix = chalk.red('⚠️');
    } else {
      titleStr = task.title.slice(0, titleWidth - 2);
      statusStr = chalk.blue('⏳ Pending');
    }

    return [
      padRight(idStr, idWidth),
      padRight(prefix + titleStr, titleWidth),
      padRight(priorityStr, priorityWidth),
      padRight(dueDateStr, dateWidth),
      statusStr,
    ].join(' ');
  });

  // Footer with count
  const footer = chalk.gray(`\n${tasks.length} task${tasks.length === 1 ? '' : 's'} total`);

  return [headerLine, separator, ...rows, footer].join('\n');
}

// =============================================================================
// Message Formatting Functions
// =============================================================================

/**
 * Format success message (green checkmark + message)
 */
export function formatSuccess(message: string): string {
  return chalk.green(`✅ ${message}`);
}

/**
 * Format error message (red X + message)
 */
export function formatError(message: string): string {
  return chalk.red(`❌ ${message}`);
}

/**
 * Format warning message (yellow warning + message)
 */
export function formatWarning(message: string): string {
  return chalk.yellow(`⚠️ ${message}`);
}

/**
 * Format info message (blue info icon + message)
 */
export function formatInfo(message: string): string {
  return chalk.blue(`ℹ️ ${message}`);
}

// =============================================================================
// Statistics Formatting
// =============================================================================

/**
 * Format statistics display
 * Shows total tasks, progress bar, completion rate, overdue count, and priority distribution
 */
export function formatStats(stats: TaskStats): string {
  const lines: string[] = [];

  // Header
  lines.push(chalk.bold.cyan('📊 Task Statistics'));
  lines.push(chalk.gray('─'.repeat(30)));
  lines.push(`Total Tasks: ${chalk.bold(stats.total.toString())}`);
  lines.push('');

  // Progress bar
  const progressBar = createProgressBar(stats.completionRate);
  lines.push(`Progress: [${progressBar}] ${chalk.bold(stats.completionRate.toString())}%`);
  lines.push(chalk.green(`✅ Completed: ${stats.completed}`));
  lines.push(chalk.blue(`⏳ Pending: ${stats.pending}`));

  // Overdue (red if > 0)
  if (stats.overdue > 0) {
    lines.push(chalk.red.bold(`⚠️ Overdue: ${stats.overdue}`));
  } else {
    lines.push(chalk.gray(`⚠️ Overdue: ${stats.overdue}`));
  }

  lines.push('');

  // Priority distribution
  lines.push(chalk.bold('Priority Distribution:'));
  lines.push(`  ${PRIORITY_ICONS[Priority.HIGH]} High:   ${stats.priorityDistribution.high}`);
  lines.push(`  ${PRIORITY_ICONS[Priority.MEDIUM]} Medium: ${stats.priorityDistribution.medium}`);
  lines.push(`  ${PRIORITY_ICONS[Priority.LOW]} Low:    ${stats.priorityDistribution.low}`);

  return lines.join('\n');
}
