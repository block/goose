type SessionScopedNotificationListener<TNotification> = (
  notification: TNotification
) => Promise<void> | void;

interface SessionScopedNotificationRouter<TNotification> {
  route(notification: TNotification): Promise<void>;
  subscribe(
    sessionId: string,
    listener: SessionScopedNotificationListener<TNotification>
  ): () => void;
}

interface SessionScopedNotification {
  sessionId: string;
}

export function createSessionScopedNotificationRouter<
  TNotification extends SessionScopedNotification,
>(): SessionScopedNotificationRouter<TNotification> {
  const listenersBySessionId = new Map<
    string,
    Set<SessionScopedNotificationListener<TNotification>>
  >();

  const addListener = (
    sessionId: string,
    listener: SessionScopedNotificationListener<TNotification>
  ): void => {
    const listeners = listenersBySessionId.get(sessionId) ?? new Set();
    listeners.add(listener);
    listenersBySessionId.set(sessionId, listeners);
  };

  const removeListener = (
    sessionId: string,
    listener: SessionScopedNotificationListener<TNotification>
  ): void => {
    const listeners = listenersBySessionId.get(sessionId);
    if (!listeners) {
      return;
    }

    listeners.delete(listener);

    if (listeners.size === 0) {
      listenersBySessionId.delete(sessionId);
    }
  };

  const route = async (notification: TNotification): Promise<void> => {
    const listeners = listenersBySessionId.get(notification.sessionId);
    if (!listeners) {
      return;
    }

    await Promise.all([...listeners].map((listener) => listener(notification)));
  };

  const subscribe = (
    sessionId: string,
    listener: SessionScopedNotificationListener<TNotification>
  ): (() => void) => {
    addListener(sessionId, listener);

    let subscribed = true;

    return () => {
      if (!subscribed) {
        return;
      }

      subscribed = false;
      removeListener(sessionId, listener);
    };
  };

  return {
    route,
    subscribe,
  };
}
