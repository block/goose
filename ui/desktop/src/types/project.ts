import { Session } from '../sessions';

/**
 * Represents a project in the system
 */
export interface Project {
  /** Unique identifier for the project */
  id: string;
  /** Display name of the project */
  name: string;
  /** Optional description of the project */
  description?: string;
  /** Default working directory for sessions in this project */
  defaultDirectory: string;
  /** When the project was created */
  createdAt: string;
  /** When the project was last updated */
  updatedAt: string;
  /** List of session IDs associated with this project */
  sessionIds: string[];
}

/**
 * Simplified project metadata for listings
 */
export interface ProjectMetadata {
  /** Unique identifier for the project */
  id: string;
  /** Display name of the project */
  name: string;
  /** Optional description of the project */
  description?: string;
  /** Default working directory for sessions in this project */
  defaultDirectory: string;
  /** Number of sessions in this project */
  sessionCount: number;
  /** When the project was created */
  createdAt: string;
  /** When the project was last updated */
  updatedAt: string;
}

/**
 * Project with associated sessions
 */
export interface ProjectWithSessions extends Project {
  /** Associated sessions */
  sessions: Session[];
}

/**
 * Request to create a new project
 */
export interface CreateProjectRequest {
  /** Display name of the project */
  name: string;
  /** Optional description of the project */
  description?: string;
  /** Default working directory for sessions in this project */
  defaultDirectory: string;
}

/**
 * Request to update an existing project
 */
export interface UpdateProjectRequest {
  /** Display name of the project */
  name?: string;
  /** Optional description of the project */
  description?: string | null;
  /** Default working directory for sessions in this project */
  defaultDirectory?: string;
}
