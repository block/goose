import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { SessionDraftStorage } from './sessionDraftStorage';

const STORAGE_KEY = 'goose-chat-drafts';

describe('SessionDraftStorage', () => {
  beforeEach(() => {
    window.sessionStorage.clear();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('returns an empty string for a key that was never written', () => {
    expect(SessionDraftStorage.get('hub')).toBe('');
  });

  it('stores and reads back a draft per key', () => {
    SessionDraftStorage.set('hub', 'unsent hub text');
    SessionDraftStorage.set('sess-1', 'unsent session text');

    expect(SessionDraftStorage.get('hub')).toBe('unsent hub text');
    expect(SessionDraftStorage.get('sess-1')).toBe('unsent session text');
  });

  it('overwrites an existing draft for the same key', () => {
    SessionDraftStorage.set('hub', 'first');
    SessionDraftStorage.set('hub', 'second');

    expect(SessionDraftStorage.get('hub')).toBe('second');
  });

  it('removes the draft when the text is empty', () => {
    SessionDraftStorage.set('hub', 'typed then deleted');
    SessionDraftStorage.set('hub', '');

    expect(SessionDraftStorage.get('hub')).toBe('');
    expect(JSON.parse(window.sessionStorage.getItem(STORAGE_KEY) as string)).toEqual({});
  });

  it('clears one key without touching the others', () => {
    SessionDraftStorage.set('hub', 'hub text');
    SessionDraftStorage.set('sess-1', 'session text');

    SessionDraftStorage.clear('hub');

    expect(SessionDraftStorage.get('hub')).toBe('');
    expect(SessionDraftStorage.get('sess-1')).toBe('session text');
  });

  it('clearing an unknown key leaves storage untouched', () => {
    SessionDraftStorage.set('sess-1', 'session text');
    const before = window.sessionStorage.getItem(STORAGE_KEY);

    SessionDraftStorage.clear('missing');

    expect(window.sessionStorage.getItem(STORAGE_KEY)).toBe(before);
  });

  it('drops the oldest drafts once the cap is reached', () => {
    for (let i = 0; i < 55; i++) {
      SessionDraftStorage.set(`sess-${i}`, `draft ${i}`);
    }

    expect(SessionDraftStorage.get('sess-0')).toBe('');
    expect(SessionDraftStorage.get('sess-4')).toBe('');
    expect(SessionDraftStorage.get('sess-5')).toBe('draft 5');
    expect(SessionDraftStorage.get('sess-54')).toBe('draft 54');
    expect(
      Object.keys(JSON.parse(window.sessionStorage.getItem(STORAGE_KEY) as string))
    ).toHaveLength(50);
  });

  it('rewriting a draft keeps it from being evicted as the oldest', () => {
    SessionDraftStorage.set('sess-0', 'first');
    for (let i = 1; i < 50; i++) {
      SessionDraftStorage.set(`sess-${i}`, `draft ${i}`);
    }
    SessionDraftStorage.set('sess-0', 'refreshed');
    SessionDraftStorage.set('sess-50', 'draft 50');

    expect(SessionDraftStorage.get('sess-0')).toBe('refreshed');
    expect(SessionDraftStorage.get('sess-1')).toBe('');
  });

  it('recovers from corrupt stored JSON instead of throwing', () => {
    vi.spyOn(console, 'error').mockImplementation(() => {});
    window.sessionStorage.setItem(STORAGE_KEY, 'not json');

    expect(SessionDraftStorage.get('hub')).toBe('');
    expect(() => SessionDraftStorage.set('hub', 'text')).not.toThrow();
    expect(SessionDraftStorage.get('hub')).toBe('text');
  });

  it('ignores stored entries that are not strings', () => {
    window.sessionStorage.setItem(STORAGE_KEY, JSON.stringify({ hub: 42, 'sess-1': 'real draft' }));

    expect(SessionDraftStorage.get('hub')).toBe('');
    expect(SessionDraftStorage.get('sess-1')).toBe('real draft');
  });

  it('ignores stored JSON that is not an object', () => {
    window.sessionStorage.setItem(STORAGE_KEY, JSON.stringify(['hub']));

    expect(SessionDraftStorage.get('hub')).toBe('');
  });

  it('does not throw when storage refuses the write', () => {
    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {});
    vi.spyOn(window.sessionStorage, 'setItem').mockImplementation(() => {
      throw new DOMException('QuotaExceededError');
    });

    expect(() => SessionDraftStorage.set('hub', 'text')).not.toThrow();
    expect(consoleError).toHaveBeenCalled();
  });
});
