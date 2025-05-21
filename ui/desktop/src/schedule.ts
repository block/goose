export interface ScheduledJob {
  id: string;
  source: string;
  cron: string;
  last_run?: string | null;
}

import { getApiUrl, getSecretKey } from './config';

export async function listSchedules(): Promise<ScheduledJob[]> {
  const response = await fetch(getApiUrl('/schedule/list'), {
    headers: { 'X-Secret-Key': getSecretKey() },
  });
  if (!response.ok) {
    throw new Error('Failed to list schedules');
  }
  const data = await response.json();
  return data.jobs as ScheduledJob[];
}

export async function createSchedule(request: {
  id: string;
  recipe_source: string;
  cron: string;
}): Promise<ScheduledJob> {
  const response = await fetch(getApiUrl('/schedule/create'), {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'X-Secret-Key': getSecretKey(),
    },
    body: JSON.stringify(request),
  });
  if (!response.ok) {
    throw new Error('Failed to create schedule');
  }
  return (await response.json()) as ScheduledJob;
}

export async function deleteSchedule(id: string): Promise<void> {
  const response = await fetch(getApiUrl(`/schedule/delete/${id}`), {
    method: 'DELETE',
    headers: { 'X-Secret-Key': getSecretKey() },
  });
  if (!response.ok) {
    throw new Error('Failed to delete schedule');
  }
}
