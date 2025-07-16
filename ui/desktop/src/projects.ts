import { Session } from './sessions';
import { client } from './api/client.gen';

/**
 * Interface for a project with all details
 */
export interface Project {
  id: string;
  name: string;
  description: string | null;
  defaultDirectory: string;
  sessionIds: string[];
  createdAt: string;
  updatedAt: string;
}

/**
 * Simplified project metadata for listings
 */
export interface ProjectMetadata {
  id: string;
  name: string;
  description: string | null;
  defaultDirectory: string;
  sessionCount: number;
  createdAt: string;
  updatedAt: string;
}

/**
 * Project with associated session objects
 */
export interface ProjectWithSessions extends Project {
  sessions: Session[];
}

/**
 * Request to create a new project
 */
export interface CreateProjectRequest {
  name: string;
  description?: string;
  defaultDirectory: string;
}

/**
 * Request to update an existing project
 */
export interface UpdateProjectRequest {
  name?: string;
  description?: string | null;
  defaultDirectory?: string;
}

/**
 * Ensure default directory is properly set
 */
function ensureDefaultDirectory(project: Partial<Project>): Project {
  return {
    id: project.id || '',
    name: project.name || '',
    description: project.description || null,
    defaultDirectory: project.defaultDirectory || process.env.HOME || '',
    sessionIds: project.sessionIds || [],
    createdAt: project.createdAt || new Date().toISOString(),
    updatedAt: project.updatedAt || new Date().toISOString(),
  };
}

/**
 * Fetches all available projects from the API
 * @returns Promise with an array of ProjectMetadata objects
 */
export async function fetchProjects(): Promise<ProjectMetadata[]> {
  try {
    const response = await client.get<{ projects: ProjectMetadata[] }>({
      url: '/projects',
    });

    if (response && response.data && response.data.projects) {
      return response.data.projects;
    } else {
      throw new Error('Unexpected response format from list_projects');
    }
  } catch (error) {
    console.error('Error fetching projects:', error);
    throw error;
  }
}

/**
 * Creates a new project
 * @param request The project creation request data
 * @returns Promise with the created project
 */
export async function createProject(request: CreateProjectRequest): Promise<Project> {
  const response = await client.post<{ project: Project }>({
    url: '/projects',
    body: request,
    headers: {
      'Content-Type': 'application/json',
    },
  });
  console.log('Raw createProject response:', response);
  return ensureDefaultDirectory(
    (response as { project?: Project }).project ?? (response as unknown as Project)
  );
}

/**
 * Gets details for a specific project
 * @param projectId The ID of the project to fetch
 * @returns Promise with project details
 */
export async function getProject(projectId: string): Promise<Project> {
  try {
    const response = await client.get<{ project: Project }>({
      url: `/projects/${projectId}`,
    });

    if (!response?.data?.project) {
      throw new Error(`Unexpected response format from get_project_details for ID: ${projectId}`);
    }

    return ensureDefaultDirectory(response.data.project);
  } catch (error) {
    console.error(`Error fetching project ${projectId}:`, error);
    throw error;
  }
}

/**
 * Updates an existing project
 * @param projectId The ID of the project to update
 * @param request The project update request data
 * @returns Promise with the updated project
 */
export async function updateProject(
  projectId: string,
  request: UpdateProjectRequest
): Promise<Project> {
  try {
    const response = await client.put<{ project: Project }>({
      url: `/projects/${projectId}`,
      body: request,
      headers: {
        'Content-Type': 'application/json',
      },
    });

    if (!response?.data?.project) {
      throw new Error(`Unexpected response format from update_project for ID: ${projectId}`);
    }

    return ensureDefaultDirectory(response.data.project);
  } catch (error) {
    console.error(`Error updating project ${projectId}:`, error);
    throw error;
  }
}

/**
 * Deletes a project
 * @param projectId The ID of the project to delete
 */
export async function deleteProject(projectId: string): Promise<void> {
  try {
    await client.delete({
      url: `/projects/${projectId}`,
    });
  } catch (error) {
    console.error(`Error deleting project ${projectId}:`, error);
    throw error;
  }
}

/**
 * Adds a session to a project
 * @param projectId The ID of the project
 * @param sessionId The ID of the session to add
 */
export async function addSessionToProject(projectId: string, sessionId: string): Promise<void> {
  try {
    await client.post({
      url: `/projects/${projectId}/sessions/${sessionId}`,
    });
  } catch (error) {
    console.error(`Error adding session ${sessionId} to project ${projectId}:`, error);
    throw error;
  }
}

/**
 * Removes a session from a project
 * @param projectId The ID of the project
 * @param sessionId The ID of the session to remove
 */
export async function removeSessionFromProject(
  projectId: string,
  sessionId: string
): Promise<void> {
  try {
    await client.delete({
      url: `/projects/${projectId}/sessions/${sessionId}`,
    });
  } catch (error) {
    console.error(`Error removing session ${sessionId} from project ${projectId}:`, error);
    throw error;
  }
}

/**
 * Generate a project ID in the format proj_yyyymmdd_hhmmss
 */
export function generateProjectId(): string {
  const now = new Date();
  const year = now.getFullYear();
  const month = String(now.getMonth() + 1).padStart(2, '0');
  const day = String(now.getDate()).padStart(2, '0');
  const hours = String(now.getHours()).padStart(2, '0');
  const minutes = String(now.getMinutes()).padStart(2, '0');
  const seconds = String(now.getSeconds()).padStart(2, '0');

  return `proj_${year}${month}${day}_${hours}${minutes}${seconds}`;
}
