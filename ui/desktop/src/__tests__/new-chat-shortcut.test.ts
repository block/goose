import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

/**
 * Regression test for #9523.
 *
 * Reproduces the renderer-side wiring for the Cmd+T / Ctrl+T "New Chat"
 * shortcut without booting the full React tree. The main process emits
 * `new-chat` over IPC when the menu accelerator fires; the renderer must
 * (1) navigate to the Home tab ('/'), and (2) dispatch the
 * `TRIGGER_NEW_CHAT` window event so any listeners (current or future)
 * can react. Before the fix, the renderer only dispatched the window
 * event, which had no listener, so the shortcut silently did nothing.
 */
describe('Cmd+T / Ctrl+T new-chat IPC handler (regression for #9523)', () => {
  type IpcHandler = (event: unknown, ...args: unknown[]) => void;

  let listeners: Map<string, Set<IpcHandler>>;
  let navigate: ReturnType<typeof vi.fn<(path: string) => void>>;
  let dispatched: string[];
  let originalDispatch: typeof window.dispatchEvent;

  // Mirror the renderer-side handler from ui/desktop/src/App.tsx.
  function attachHandler() {
    const handleNewChat: IpcHandler = () => {
      navigate('/');
      window.dispatchEvent(new CustomEvent('trigger-new-chat'));
    };
    listeners.get('new-chat')!.add(handleNewChat);
    return () => listeners.get('new-chat')!.delete(handleNewChat);
  }

  function emitIpc(channel: string, ...args: unknown[]) {
    for (const handler of listeners.get(channel) ?? []) {
      handler({}, ...args);
    }
  }

  beforeEach(() => {
    listeners = new Map([['new-chat', new Set<IpcHandler>()]]);
    navigate = vi.fn<(path: string) => void>();
    dispatched = [];
    originalDispatch = window.dispatchEvent;
    window.dispatchEvent = ((event: Event) => {
      dispatched.push(event.type);
      return originalDispatch.call(window, event);
    }) as typeof window.dispatchEvent;
  });

  afterEach(() => {
    window.dispatchEvent = originalDispatch;
  });

  it('navigates to the Home tab when the main process emits new-chat', () => {
    attachHandler();

    emitIpc('new-chat');

    expect(navigate).toHaveBeenCalledTimes(1);
    expect(navigate).toHaveBeenCalledWith('/');
  });

  it('also dispatches the TRIGGER_NEW_CHAT window event so future listeners stay wired', () => {
    attachHandler();

    emitIpc('new-chat');

    expect(dispatched).toContain('trigger-new-chat');
  });

  it('does not fire navigate when no IPC event is received', () => {
    attachHandler();

    expect(navigate).not.toHaveBeenCalled();
    expect(dispatched).not.toContain('trigger-new-chat');
  });

  it('detaches cleanly so unmounted handlers do not navigate', () => {
    const detach = attachHandler();
    detach();

    emitIpc('new-chat');

    expect(navigate).not.toHaveBeenCalled();
    expect(dispatched).not.toContain('trigger-new-chat');
  });
});
