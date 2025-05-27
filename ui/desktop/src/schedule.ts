import {
  listSchedules as apiListSchedules,
  createSchedule as apiCreateSchedule,
  deleteSchedule as apiDeleteSchedule,
  sessionsHandler as apiGetScheduleSessions,
  runNowHandler as apiRunScheduleNow,
} from './api';

export interface ScheduledJob {
  id: string;
  source: string;
  cron: string;
  last_run?: string | null;
}

export interface ScheduleSession {
  id: string;
  name: string;
  createdAt: string; // ISO 8601 date string
  workingDir: string;
  scheduleId: string;
  messageCount: number;
  totalTokens: number;
  inputTokens: number;
  outputTokens: number;
  accumulatedTotalTokens: number;
  accumulatedInputTokens: number;
  accumulatedOutputTokens: number;
}

export async function listSchedules(): Promise<ScheduledJob[]> {
  try {
    const response = await apiListSchedules<true>();
    if (response && response.data && Array.isArray(response.data.jobs)) {
      return response.data.jobs as ScheduledJob[];
    }
    console.error('Unexpected response format from apiListSchedules', response);
    throw new Error('Failed to list schedules: Unexpected response format');
  } catch (error) {
    console.error('Error listing schedules:', error);
    throw error;
  }
}

export async function createSchedule(request: {
  id: string;
  recipe_source: string;
  cron: string;
}): Promise<ScheduledJob> {
  try {
    const response = await apiCreateSchedule<true>({ data: request });
    if (response && response.data) {
      return response.data as ScheduledJob;
    }
    console.error('Unexpected response format from apiCreateSchedule', response);
    throw new Error('Failed to create schedule: Unexpected response format');
  } catch (error) {
    console.error('Error creating schedule:', error);
    throw error;
  }
}

export async function deleteSchedule(id: string): Promise<void> {
  try {
    await apiDeleteSchedule<true>({ path: { schedule_id: id } });
  } catch (error) {
    console.error(`Error deleting schedule ${id}:`, error);
    throw error;
  }
}

export async function getScheduleSessions(
  scheduleId: string,
  limit?: number
): Promise<ScheduleSession[]> {
  try {
    const response = await apiGetScheduleSessions<true>({
      path: { id: scheduleId },
      query: { limit },
    });

    if (response && response.data) {
      return response.data as ScheduleSession[];
    }
    console.error('Unexpected response format from apiGetScheduleSessions', response);
    throw new Error('Failed to get schedule sessions: Unexpected response format');
  } catch (error) {
    console.error(`Error fetching sessions for schedule ${scheduleId}:`, error);
    throw error;
  }
}

export async function runScheduleNow(scheduleId: string): Promise<string> {
  try {
    const response = await apiRunScheduleNow<true>({
      path: { id: scheduleId },
    });

    if (response && response.data && response.data.session_id) {
      return response.data.session_id;
    }
    console.error('Unexpected response format from apiRunScheduleNow', response);
    throw new Error('Failed to run schedule now: Unexpected response format');
  } catch (error) {
    console.error(`Error running schedule ${scheduleId} now:`, error);
    throw error;
  }
}
