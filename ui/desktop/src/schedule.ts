import type { Recipe, SessionDisplayInfo } from './api';
import {
  createSchedule as apiCreateSchedule,
  deleteSchedule as apiDeleteSchedule,
  sessionsHandler as apiGetScheduleSessions,
  inspectRunningJob as apiInspectRunningJob,
  killRunningJob as apiKillRunningJob,
  listSchedules as apiListSchedules,
  pauseSchedule as apiPauseSchedule,
  runNowHandler as apiRunScheduleNow,
  unpauseSchedule as apiUnpauseSchedule,
  updateSchedule as apiUpdateSchedule,
} from './api';

export interface ScheduledJob {
  id: string;
  source: string;
  cron: string;
  last_run?: string | null;
  currently_running?: boolean;
  paused?: boolean;
  current_session_id?: string | null;
  process_start_time?: string | null;
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
    if (response?.data && Array.isArray(response.data.jobs)) {
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
  recipe: Recipe;
  cron: string;
}): Promise<ScheduledJob> {
  const response = await apiCreateSchedule({ body: request });
  if (response.data) {
    return response.data as ScheduledJob;
  }
  const err = response.error as { message?: string } | undefined;
  throw new Error(err?.message || 'Failed to create schedule');
}

export async function deleteSchedule(id: string): Promise<void> {
  try {
    await apiDeleteSchedule<true>({ path: { id } });
  } catch (error) {
    console.error(`Error deleting schedule ${id}:`, error);
    throw error;
  }
}

export async function getScheduleSessions(
  scheduleId: string,
  limit: number
): Promise<Array<SessionDisplayInfo>> {
  const response = await apiGetScheduleSessions<true>({
    path: { id: scheduleId },
    query: { limit },
    throwOnError: true,
  });

  return response.data;
}

export async function runScheduleNow(scheduleId: string): Promise<string> {
  try {
    const response = await apiRunScheduleNow<true>({
      path: { id: scheduleId },
    });

    if (response?.data?.session_id) {
      return response.data.session_id;
    }
    console.error('Unexpected response format from apiRunScheduleNow', response);
    throw new Error('Failed to run schedule now: Unexpected response format');
  } catch (error) {
    console.error(`Error running schedule ${scheduleId} now:`, error);
    throw error;
  }
}

export async function pauseSchedule(scheduleId: string): Promise<void> {
  try {
    await apiPauseSchedule<true>({
      path: { id: scheduleId },
    });
  } catch (error) {
    console.error(`Error pausing schedule ${scheduleId}:`, error);
    throw error;
  }
}

export async function unpauseSchedule(scheduleId: string): Promise<void> {
  try {
    await apiUnpauseSchedule<true>({
      path: { id: scheduleId },
    });
  } catch (error) {
    console.error(`Error unpausing schedule ${scheduleId}:`, error);
    throw error;
  }
}

export async function updateSchedule(scheduleId: string, cron: string): Promise<ScheduledJob> {
  try {
    const response = await apiUpdateSchedule<true>({
      path: { id: scheduleId },
      body: { cron },
    });

    if (response?.data) {
      return response.data as ScheduledJob;
    }
    console.error('Unexpected response format from apiUpdateSchedule', response);
    throw new Error('Failed to update schedule: Unexpected response format');
  } catch (error) {
    console.error(`Error updating schedule ${scheduleId}:`, error);
    throw error;
  }
}

export interface KillJobResponse {
  message: string;
}

export interface InspectJobResponse {
  sessionId?: string | null;
  processStartTime?: string | null;
  runningDurationSeconds?: number | null;
}

export async function killRunningJob(scheduleId: string): Promise<KillJobResponse> {
  try {
    const response = await apiKillRunningJob<true>({
      path: { id: scheduleId },
    });

    if (response?.data) {
      return response.data as KillJobResponse;
    }
    console.error('Unexpected response format from apiKillRunningJob', response);
    throw new Error('Failed to kill running job: Unexpected response format');
  } catch (error) {
    console.error(`Error killing running job ${scheduleId}:`, error);
    throw error;
  }
}

export async function inspectRunningJob(scheduleId: string): Promise<InspectJobResponse> {
  try {
    const response = await apiInspectRunningJob<true>({
      path: { id: scheduleId },
    });

    if (response?.data) {
      return response.data as InspectJobResponse;
    }
    console.error('Unexpected response format from apiInspectRunningJob', response);
    throw new Error('Failed to inspect running job: Unexpected response format');
  } catch (error) {
    console.error(`Error inspecting running job ${scheduleId}:`, error);
    throw error;
  }
}
