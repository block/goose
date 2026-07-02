import { describe, expect, it } from 'vitest';
import {
  getProjectLabel,
  groupSessionsByProject,
} from '../utils/projectSessions';
import type { SessionListItem } from '../acp/sessions';

function makeSession(overrides: Partial<SessionListItem> = {}): SessionListItem {
  return {
    id: 'session-1',
    name: 'Session',
    messageCount: 1,
    createdAt: '2026-01-01T00:00:00.000Z',
    updatedAt: '2026-01-01T00:00:00.000Z',
    workingDir: '/tmp/goose',
    ...overrides,
  };
}

describe('groupSessionsByProject', () => {
  it('groups sessions with the same working directory', () => {
    const groups = groupSessionsByProject([
      makeSession({ id: 'a', workingDir: '/tmp/goose' }),
      makeSession({ id: 'b', workingDir: '/tmp/goose' }),
      makeSession({ id: 'c', workingDir: '/tmp/other' }),
    ]);

    expect(groups).toHaveLength(2);
    expect(groups.find((group) => group.path === '/tmp/goose')?.sessions).toHaveLength(2);
    expect(groups.find((group) => group.path === '/tmp/other')?.sessions).toHaveLength(1);
  });

  it('sorts project groups by most recent session', () => {
    const groups = groupSessionsByProject([
      makeSession({ id: 'old', workingDir: '/tmp/old', updatedAt: '2026-01-01T00:00:00.000Z' }),
      makeSession({ id: 'new', workingDir: '/tmp/new', updatedAt: '2026-01-03T00:00:00.000Z' }),
      makeSession({
        id: 'middle',
        workingDir: '/tmp/middle',
        updatedAt: '2026-01-02T00:00:00.000Z',
      }),
    ]);

    expect(groups.map((group) => group.path)).toEqual(['/tmp/new', '/tmp/middle', '/tmp/old']);
  });

  it('sorts sessions within each project newest first', () => {
    const groups = groupSessionsByProject([
      makeSession({ id: 'old', updatedAt: '2026-01-01T00:00:00.000Z' }),
      makeSession({ id: 'new', updatedAt: '2026-01-03T00:00:00.000Z' }),
      makeSession({ id: 'middle', updatedAt: '2026-01-02T00:00:00.000Z' }),
    ]);

    expect(groups[0].sessions.map((session) => session.id)).toEqual(['new', 'middle', 'old']);
  });

  it('canonicalizes trailing separators when grouping projects', () => {
    const groups = groupSessionsByProject([
      makeSession({ id: 'a', workingDir: '/tmp/goose' }),
      makeSession({ id: 'b', workingDir: '/tmp/goose/' }),
      makeSession({ id: 'c', workingDir: '  /tmp/goose//  ' }),
    ]);

    expect(groups).toHaveLength(1);
    expect(groups[0].path).toBe('/tmp/goose');
    expect(groups[0].sessions.map((session) => session.id)).toEqual(['a', 'b', 'c']);
  });

  it('normalizes empty working directories into one group', () => {
    const groups = groupSessionsByProject([
      makeSession({ id: 'a', workingDir: '' }),
      makeSession({ id: 'b', workingDir: '   ' }),
    ]);

    expect(groups).toHaveLength(1);
    expect(groups[0].path).toBe('');
    expect(groups[0].label).toBe('Unknown');
    expect(groups[0].sessions).toHaveLength(2);
  });

  it('returns an empty array for empty input', () => {
    expect(groupSessionsByProject([])).toEqual([]);
  });

  it('disambiguates projects with the same basename', () => {
    const groups = groupSessionsByProject([
      makeSession({ id: 'a', workingDir: '/Users/me/work/goose' }),
      makeSession({ id: 'b', workingDir: '/Users/me/forks/goose' }),
    ]);

    expect(groups.map((group) => group.label).sort()).toEqual(['forks/goose', 'work/goose']);
  });
});

describe('getProjectLabel', () => {
  it('extracts the basename from an absolute path', () => {
    expect(getProjectLabel('/Users/me/work/goose')).toBe('goose');
  });

  it('handles the root path', () => {
    expect(getProjectLabel('/')).toBe('/');
  });

  it('handles an empty path', () => {
    expect(getProjectLabel('')).toBe('Unknown');
  });

  it('handles Windows-style paths', () => {
    expect(getProjectLabel('C:\\Users\\me\\goose')).toBe('goose');
  });
});
