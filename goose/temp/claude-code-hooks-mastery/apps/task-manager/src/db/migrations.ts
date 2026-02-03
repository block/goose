/**
 * Database migrations for the Task Manager
 */

import { Database } from 'bun:sqlite';

/**
 * Run all database migrations
 * Creates the tasks table if it doesn't exist
 */
export function runMigrations(db: Database): void {
  // Create tasks table if not exists
  db.run(`
    CREATE TABLE IF NOT EXISTS tasks (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      title TEXT NOT NULL,
      description TEXT,
      priority TEXT DEFAULT 'medium',
      due_date TEXT,
      completed INTEGER DEFAULT 0,
      created_at TEXT NOT NULL,
      completed_at TEXT
    )
  `);

  // Create index on priority for filtering
  db.run(`
    CREATE INDEX IF NOT EXISTS idx_tasks_priority ON tasks(priority)
  `);

  // Create index on completed for filtering
  db.run(`
    CREATE INDEX IF NOT EXISTS idx_tasks_completed ON tasks(completed)
  `);

  // Create index on due_date for overdue queries
  db.run(`
    CREATE INDEX IF NOT EXISTS idx_tasks_due_date ON tasks(due_date)
  `);
}
