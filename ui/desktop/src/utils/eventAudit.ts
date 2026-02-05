import { trackErrorWithContext } from './analytics';

type EventHandler<E> = ((event: E) => void | Promise<void>) | undefined;

export type EventAuditContext = {
  component: string;
  action?: string;
  recoverable?: boolean;
};

export function guardEventHandler<E>(
  handler: EventHandler<E>,
  context: EventAuditContext
): ((event: E) => void) | undefined {
  if (!handler) {
    return undefined;
  }

  return (event: E) => {
    try {
      const result = handler(event);
      if (result && typeof (result as Promise<void>).catch === 'function') {
        (result as Promise<void>).catch((error) => {
          console.error('[EventAudit] Handler rejected:', error);
          trackErrorWithContext(error, {
            component: context.component,
            action: context.action ?? 'event_handler',
            recoverable: context.recoverable ?? true,
          });
        });
      }
    } catch (error) {
      console.error('[EventAudit] Handler threw:', error);
      trackErrorWithContext(error, {
        component: context.component,
        action: context.action ?? 'event_handler',
        recoverable: context.recoverable ?? true,
      });
    }
  };
}
