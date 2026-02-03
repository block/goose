/**
 * TypeScript types and interfaces for the Task Manager CLI
 */

// =============================================================================
// Enums
// =============================================================================

export enum Priority {
  LOW = 'low',
  MEDIUM = 'medium',
  HIGH = 'high'
}

// =============================================================================
// Core Data Types
// =============================================================================

export interface Task {
  id: number;
  title: string;
  description: string | null;
  priority: Priority;
  dueDate: string | null;  // ISO date string YYYY-MM-DD
  completed: boolean;
  createdAt: string;  // ISO timestamp
  completedAt: string | null;  // ISO timestamp
}

export interface TaskStats {
  total: number;
  completed: number;
  pending: number;
  overdue: number;
  completionRate: number;  // 0-100 percentage
  priorityDistribution: {
    high: number;
    medium: number;
    low: number;
  };
}

// =============================================================================
// CLI Command Argument Types
// =============================================================================

export interface AddCommandArgs {
  title: string;
  description?: string;
  priority?: string;
  due?: string;
}

export interface ListCommandArgs {
  priority?: string;
  completed?: boolean;
  pending?: boolean;
  overdue?: boolean;
}

export interface CompleteCommandArgs {
  id: number;
}

export interface DeleteCommandArgs {
  id: number;
  force?: boolean;
}

// =============================================================================
// Database Operation Types
// =============================================================================

export interface CreateTaskInput {
  title: string;
  description?: string;
  priority: Priority;
  dueDate?: string;
}

// =============================================================================
// Validation Types
// =============================================================================

export interface ValidationResult {
  valid: boolean;
  error?: string;
}
