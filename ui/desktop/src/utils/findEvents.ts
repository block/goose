// Global bridge for "Find" menu actions.
//
// Why: multiple SearchView instances across the app were each registering
// ipcRenderer listeners, eventually triggering MaxListenersExceededWarning.
//
// This module registers *one* set of Electron IPC listeners per renderer
// process and re-emits them as DOM events.

const EVENT_PREFIX = 'goose:find:' as const;

type FindEventName = 'command' | 'next' | 'previous' | 'use-selection';

type FindDomEventName = `${typeof EVENT_PREFIX}${FindEventName}`;

const domEvent = (name: FindEventName): FindDomEventName => `${EVENT_PREFIX}${name}`;

let initialized = false;

export function initFindEvents(): void {
  if (initialized) return;
  initialized = true;

  if (typeof window === 'undefined') return;
  if (!window.electron?.on) return;

  window.electron.on('find-command', () => window.dispatchEvent(new Event(domEvent('command'))));
  window.electron.on('find-next', () => window.dispatchEvent(new Event(domEvent('next'))));
  window.electron.on('find-previous', () => window.dispatchEvent(new Event(domEvent('previous'))));
  window.electron.on('use-selection-find', () =>
    window.dispatchEvent(new Event(domEvent('use-selection')))
  );
}

export function onFindEvent(name: FindEventName, handler: () => void): () => void {
  initFindEvents();

  const type = domEvent(name);
  window.addEventListener(type, handler);

  return () => {
    window.removeEventListener(type, handler);
  };
}
