import type {
  SessionActivityDay as WireSessionActivityDay,
  SessionActivityModel as WireSessionActivityModel,
  SessionActivityResponse_unstable,
  SessionActivitySession as WireSessionActivitySession,
} from '@aaif/goose-sdk';
import { getAcpClient } from './acpConnection';

export type SessionActivitySession = WireSessionActivitySession;
export type SessionActivityModel = WireSessionActivityModel;
export type SessionActivityDay = Omit<WireSessionActivityDay, 'sessions'> & {
  sessions: SessionActivitySession[];
};
export type SessionActivity = Omit<SessionActivityResponse_unstable, 'days' | 'models'> & {
  days: SessionActivityDay[];
  models: SessionActivityModel[];
};

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
      ...day,
      sessions: day.sessions ?? [],
    })),
    models: response.models ?? [],
  };
}
