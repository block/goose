import { getAcpClient } from './acpConnection';

export interface SessionActivitySession {
  id: string;
  name: string;
  totalTokens: number;
  providerId?: string | null;
  modelId?: string | null;
}

export interface SessionActivityDay {
  date: string;
  sessionCount: number;
  totalTokens: number;
  sessions: SessionActivitySession[];
}

export interface SessionActivityModel {
  providerId?: string | null;
  modelId?: string | null;
  totalTokens: number;
  sessionCount: number;
}

export interface SessionActivity {
  year: number;
  totalTokens: number;
  totalSessions: number;
  days: SessionActivityDay[];
  models: SessionActivityModel[];
}

export async function acpGetSessionActivity(year?: number): Promise<SessionActivity> {
  const client = await getAcpClient();
  const response = await client.goose.sessionsActivity_unstable(
    year === undefined ? {} : { year }
  );

  return {
    year: response.year,
    totalTokens: response.totalTokens,
    totalSessions: Number(response.totalSessions),
    days: (response.days ?? []).map((day) => ({
      date: day.date,
      sessionCount: day.sessionCount,
      totalTokens: day.totalTokens,
      sessions: (day.sessions ?? []).map((session) => ({
        id: session.id,
        name: session.name,
        totalTokens: session.totalTokens,
        providerId: session.providerId,
        modelId: session.modelId,
      })),
    })),
    models: (response.models ?? []).map((model) => ({
      providerId: model.providerId,
      modelId: model.modelId,
      totalTokens: model.totalTokens,
      sessionCount: model.sessionCount,
    })),
  };
}
