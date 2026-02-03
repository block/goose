/**
 * Stats command - Show task statistics
 */

import { TaskRepository } from '../db/repository';
import { formatStats, formatInfo } from '../utils/formatter';

export const command = 'stats';
export const describe = 'Show task statistics';
export const builder = {};

export async function handler(): Promise<void> {
  const repository = new TaskRepository();

  try {
    const stats = repository.getStats();

    if (stats.total === 0) {
      console.log(formatInfo('No tasks yet. Add some tasks to see statistics!'));
    } else {
      console.log(formatStats(stats));
    }
  } finally {
    repository.close();
  }
}
