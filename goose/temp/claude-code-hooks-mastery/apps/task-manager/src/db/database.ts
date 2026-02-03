/**
 * Database connection management for the Task Manager
 */

import { Database } from 'bun:sqlite';
import { mkdirSync, existsSync } from 'fs';
import { dirname, resolve } from 'path';
import { runMigrations } from './migrations';

// Store DB at ./data/tasks.db (relative to task-manager directory)
const DB_PATH = resolve(import.meta.dir, '../../data/tasks.db');

// Track if migrations have been run this session
let migrationsRun = false;

/**
 * Get a database connection
 * Creates the data directory and runs migrations if needed
 */
export function getDatabase(): Database {
  try {
    // Ensure data directory exists
    const dataDir = dirname(DB_PATH);
    if (!existsSync(dataDir)) {
      mkdirSync(dataDir, { recursive: true });
    }

    // Create database connection
    const db = new Database(DB_PATH);

    // Enable WAL mode for better performance
    db.run('PRAGMA journal_mode = WAL');

    // Run migrations on first connection
    if (!migrationsRun) {
      runMigrations(db);
      migrationsRun = true;
    }

    return db;
  } catch (error) {
    const message = error instanceof Error ? error.message : 'Unknown error';
    throw new Error(`Failed to connect to database: ${message}`);
  }
}

/**
 * Close a database connection
 */
export function closeDatabase(db: Database): void {
  try {
    db.close();
  } catch (error) {
    // Ignore errors when closing
  }
}

/**
 * Get the database file path (useful for testing/debugging)
 */
export function getDatabasePath(): string {
  return DB_PATH;
}
