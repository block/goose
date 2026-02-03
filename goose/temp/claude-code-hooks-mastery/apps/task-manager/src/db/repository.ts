/**
 * Task Repository - Data access layer for tasks
 */

import { Database } from 'bun:sqlite';
import { Task, CreateTaskInput, TaskStats, Priority } from '../types';
import { getDatabase, closeDatabase } from './database';

/**
 * Database row type (raw from SQLite)
 */
interface TaskRow {
  id: number;
  title: string;
  description: string | null;
  priority: string;
  due_date: string | null;
  completed: number;
  created_at: string;
  completed_at: string | null;
}

/**
 * Helper to map database row to Task interface
 */
function mapRowToTask(row: TaskRow): Task {
  return {
    id: row.id,
    title: row.title,
    description: row.description,
    priority: row.priority as Priority,
    dueDate: row.due_date,
    completed: row.completed === 1,
    createdAt: row.created_at,
    completedAt: row.completed_at,
  };
}

/**
 * Get today's date as ISO string (YYYY-MM-DD)
 */
function getTodayDate(): string {
  return new Date().toISOString().split('T')[0];
}

/**
 * Task Repository class for database operations
 */
export class TaskRepository {
  private db: Database;

  constructor() {
    this.db = getDatabase();
  }

  /**
   * Create a new task
   * @param input - Task creation input
   * @returns The created task
   */
  create(input: CreateTaskInput): Task {
    const now = new Date().toISOString();

    try {
      const stmt = this.db.prepare(`
        INSERT INTO tasks (title, description, priority, due_date, completed, created_at)
        VALUES (?, ?, ?, ?, 0, ?)
      `);

      const result = stmt.run(
        input.title,
        input.description ?? null,
        input.priority,
        input.dueDate ?? null,
        now
      );

      const id = Number(result.lastInsertRowid);

      // Fetch and return the created task
      const task = this.findById(id);
      if (!task) {
        throw new Error('Failed to retrieve created task');
      }

      return task;
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      throw new Error(`Failed to create task: ${message}`);
    }
  }

  /**
   * Find all tasks with optional filters
   * @param filters - Optional filters for priority, completed status, pending, overdue
   * @returns Array of tasks matching the filters
   */
  findAll(filters?: {
    priority?: Priority;
    completed?: boolean;
    pending?: boolean;
    overdue?: boolean;
  }): Task[] {
    try {
      let query = 'SELECT * FROM tasks WHERE 1=1';
      const params: (string | number)[] = [];

      if (filters) {
        // Priority filter
        if (filters.priority) {
          query += ' AND priority = ?';
          params.push(filters.priority);
        }

        // Completed filter (explicit true)
        if (filters.completed === true) {
          query += ' AND completed = 1';
        }

        // Pending filter (not completed)
        if (filters.pending === true) {
          query += ' AND completed = 0';
        }

        // Overdue filter: due_date < today AND not completed
        if (filters.overdue === true) {
          const today = getTodayDate();
          query += ' AND due_date < ? AND due_date IS NOT NULL AND completed = 0';
          params.push(today);
        }
      }

      query += ' ORDER BY created_at DESC';

      const stmt = this.db.prepare(query);
      const rows = stmt.all(...params) as TaskRow[];

      return rows.map(mapRowToTask);
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      throw new Error(`Failed to fetch tasks: ${message}`);
    }
  }

  /**
   * Find a single task by ID
   * @param id - Task ID
   * @returns The task or null if not found
   */
  findById(id: number): Task | null {
    try {
      const stmt = this.db.prepare('SELECT * FROM tasks WHERE id = ?');
      const row = stmt.get(id) as TaskRow | null;

      if (!row) {
        return null;
      }

      return mapRowToTask(row);
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      throw new Error(`Failed to find task: ${message}`);
    }
  }

  /**
   * Update a task
   * @param id - Task ID
   * @param updates - Partial task updates
   * @returns The updated task or null if not found
   */
  update(id: number, updates: Partial<Task>): Task | null {
    try {
      // Check if task exists
      const existing = this.findById(id);
      if (!existing) {
        return null;
      }

      // Build dynamic update query
      const setClauses: string[] = [];
      const params: (string | number | null)[] = [];

      if (updates.title !== undefined) {
        setClauses.push('title = ?');
        params.push(updates.title);
      }

      if (updates.description !== undefined) {
        setClauses.push('description = ?');
        params.push(updates.description);
      }

      if (updates.priority !== undefined) {
        setClauses.push('priority = ?');
        params.push(updates.priority);
      }

      if (updates.dueDate !== undefined) {
        setClauses.push('due_date = ?');
        params.push(updates.dueDate);
      }

      if (updates.completed !== undefined) {
        setClauses.push('completed = ?');
        params.push(updates.completed ? 1 : 0);

        // Set completed_at when marking as complete
        if (updates.completed) {
          setClauses.push('completed_at = ?');
          params.push(new Date().toISOString());
        } else {
          setClauses.push('completed_at = NULL');
        }
      }

      if (updates.completedAt !== undefined) {
        setClauses.push('completed_at = ?');
        params.push(updates.completedAt);
      }

      if (setClauses.length === 0) {
        return existing; // No updates to apply
      }

      params.push(id);

      const stmt = this.db.prepare(`
        UPDATE tasks SET ${setClauses.join(', ')} WHERE id = ?
      `);

      stmt.run(...params);

      return this.findById(id);
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      throw new Error(`Failed to update task: ${message}`);
    }
  }

  /**
   * Delete a task by ID
   * @param id - Task ID
   * @returns true if deleted, false if not found
   */
  delete(id: number): boolean {
    try {
      const stmt = this.db.prepare('DELETE FROM tasks WHERE id = ?');
      const result = stmt.run(id);

      return result.changes > 0;
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      throw new Error(`Failed to delete task: ${message}`);
    }
  }

  /**
   * Get task statistics
   * @returns TaskStats object with counts and distribution
   */
  getStats(): TaskStats {
    try {
      const today = getTodayDate();

      // Get total count
      const totalStmt = this.db.prepare('SELECT COUNT(*) as count FROM tasks');
      const totalResult = totalStmt.get() as { count: number };
      const total = totalResult.count;

      // Get completed count
      const completedStmt = this.db.prepare('SELECT COUNT(*) as count FROM tasks WHERE completed = 1');
      const completedResult = completedStmt.get() as { count: number };
      const completed = completedResult.count;

      // Get pending count
      const pending = total - completed;

      // Get overdue count
      const overdueStmt = this.db.prepare(`
        SELECT COUNT(*) as count FROM tasks
        WHERE due_date < ? AND due_date IS NOT NULL AND completed = 0
      `);
      const overdueResult = overdueStmt.get(today) as { count: number };
      const overdue = overdueResult.count;

      // Get priority distribution
      const priorityStmt = this.db.prepare(`
        SELECT priority, COUNT(*) as count FROM tasks GROUP BY priority
      `);
      const priorityRows = priorityStmt.all() as { priority: string; count: number }[];

      const priorityDistribution = {
        high: 0,
        medium: 0,
        low: 0,
      };

      for (const row of priorityRows) {
        if (row.priority === Priority.HIGH) {
          priorityDistribution.high = row.count;
        } else if (row.priority === Priority.MEDIUM) {
          priorityDistribution.medium = row.count;
        } else if (row.priority === Priority.LOW) {
          priorityDistribution.low = row.count;
        }
      }

      // Calculate completion rate
      const completionRate = total > 0 ? Math.round((completed / total) * 100) : 0;

      return {
        total,
        completed,
        pending,
        overdue,
        completionRate,
        priorityDistribution,
      };
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      throw new Error(`Failed to get stats: ${message}`);
    }
  }

  /**
   * Close the database connection
   */
  close(): void {
    closeDatabase(this.db);
  }
}
