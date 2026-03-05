import { describe, expect, it } from 'vitest';
import { initialState, type StreamAction, streamReducer } from '@/hooks/chatStream/streamReducer';
import { ChatState } from '@/types/chatState';
import type { NotificationEvent } from '@/types/message';

function notif(message: Record<string, unknown>, request_id = 'r1'): NotificationEvent {
  return {
    type: 'Notification',
    request_id,
    message,
  } as unknown as NotificationEvent;
}

describe('streamReducer notifications vs activityEvents', () => {
  it('routes activity events into activityEvents via ADD_ACTIVITY_EVENT', () => {
    const activity = notif({
      method: 'goose/activity',
      params: { phase: 'render', text: 'Generating chart…' },
    });

    const next = streamReducer(initialState, { type: 'ADD_ACTIVITY_EVENT', payload: activity });

    expect(next.activityEvents).toHaveLength(1);
    expect(next.notifications).toHaveLength(0);
  });

  it('stores non-activity notifications in notifications via ADD_NOTIFICATION', () => {
    const n = notif({ method: 'notifications/progress', params: { progress: 0.5 } }, 'r2');

    const next = streamReducer(initialState, { type: 'ADD_NOTIFICATION', payload: n });

    expect(next.notifications).toHaveLength(1);
    expect(next.activityEvents).toHaveLength(0);
  });

  it('START_STREAMING clears notifications but preserves activityEvents', () => {
    const activity = notif({
      method: 'goose/activity',
      params: { phase: 'routing', text: 'Switched' },
    });
    const other = notif({ method: 'notifications/message', params: { message: 'hi' } }, 'r3');

    const seeded = {
      ...initialState,
      notifications: [other],
      activityEvents: [activity],
    };

    const next = streamReducer(seeded, { type: 'START_STREAMING' });

    expect(next.chatState).toBe(ChatState.Streaming);
    expect(next.notifications).toHaveLength(0);
    expect(next.activityEvents).toHaveLength(1);
  });

  it('RESET_FOR_NEW_SESSION clears both notifications and activityEvents', () => {
    const seeded = {
      ...initialState,
      notifications: [notif({ method: 'notifications/message', params: {} })],
      activityEvents: [notif({ method: 'goose/activity', params: {} })],
    };

    const next = streamReducer(seeded, { type: 'RESET_FOR_NEW_SESSION' } as StreamAction);

    expect(next.notifications).toHaveLength(0);
    expect(next.activityEvents).toHaveLength(0);
  });
});
