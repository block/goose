import { describe, it, expect } from 'vitest';
import { isSchedulerSession } from './SessionListView';

// Helper to create test session objects with required properties
const createTestSession = (id: string, metadata: Record<string, unknown>) => ({
  id,
  path: '/test/path',
  modified: '2023-01-01T00:00:00Z',
  metadata: {
    message_count: 0,
    working_dir: '/test/dir',
    description: 'test description',
    schedule_id: null,
    ...metadata
  }
});

describe('SessionListView - isSchedulerSession', () => {
  describe('schedule_id detection', () => {
    it('should identify scheduler sessions when schedule_id is present', () => {
      const session = createTestSession('regular-session-id', {
        schedule_id: 'some-schedule-id',
        description: 'Some description'
      });
      expect(isSchedulerSession(session)).toBe(true);
    });

    it('should not identify sessions when schedule_id is null', () => {
      const session = createTestSession('regular-session-id', {
        schedule_id: null,
        description: 'Some description'
      });
      expect(isSchedulerSession(session)).toBe(false);
    });

    it('should not identify sessions when schedule_id is undefined', () => {
      const session = createTestSession('regular-session-id', {
        description: 'Some description'
      });
      expect(isSchedulerSession(session)).toBe(false);
    });

    it('should identify sessions when schedule_id is empty string', () => {
      const session = createTestSession('regular-session-id', {
        schedule_id: 'some-schedule-id',
        description: 'Some description'
      });
      expect(isSchedulerSession(session)).toBe(true);
    });
  });

  describe('empty description detection', () => {
    it('should identify scheduler sessions when description is null', () => {
      const session = createTestSession('scheduler-session-id', {
        schedule_id: null,
        description: null
      });
      expect(isSchedulerSession(session)).toBe(true);
    });

    it('should identify scheduler sessions when description is empty string', () => {
      const session = createTestSession('scheduler-session-id', {
        schedule_id: null,
        description: ''
      });
      expect(isSchedulerSession(session)).toBe(true);
    });

    it('should identify scheduler sessions when description is whitespace only', () => {
      const session = createTestSession('scheduler-session-id', {
        schedule_id: null,
        description: '   '
      });
      expect(isSchedulerSession(session)).toBe(true);
    });

    it('should identify scheduler sessions when description is undefined', () => {
      const session = createTestSession('scheduler-session-id', {
        schedule_id: null,
        description: undefined
      });
      expect(isSchedulerSession(session)).toBe(true);
    });

    it('should not identify sessions when description has content', () => {
      const session = createTestSession('regular-session-id', {
        schedule_id: null,
        description: 'User-provided description'
      });
      expect(isSchedulerSession(session)).toBe(false);
    });
  });

  describe('timestamp pattern detection', () => {
    it('should identify scheduler sessions when ID matches description and follows timestamp pattern', () => {
      const validTimestamps = [
        '20250820_143000',
        '20231225_120000',
        '20240229_235959'
      ];

      validTimestamps.forEach((timestamp) => {
        const session = createTestSession(timestamp, {
          schedule_id: null,
          description: timestamp
        });
        expect(isSchedulerSession(session)).toBe(true);
      });
    });

    it('should not identify sessions when ID matches description but pattern is invalid', () => {
      const invalidPatterns = [
        '2025820_143000',   // Missing zero in date
        '20250820_14300',   // Missing digit in time
        '20250820143000',   // Missing underscore
        '20250820_1430000', // Extra digit in time
        'abc20250820_143000', // Extra characters
        '20250820_143000def'  // Extra characters
      ];

      invalidPatterns.forEach((pattern) => {
        const session = createTestSession(pattern, {
          schedule_id: null,
          description: pattern
        });
        expect(isSchedulerSession(session)).toBe(false);
      });
    });

    it('should not identify sessions when ID does not match description', () => {
      const session = createTestSession('20250820_143000', {
        schedule_id: null,
        description: 'Different description'
      });
      expect(isSchedulerSession(session)).toBe(false);
    });

    it('should not identify sessions when description matches pattern but ID does not', () => {
      const session = createTestSession('different-id', {
        schedule_id: null,
        description: '20250820_143000'
      });
      expect(isSchedulerSession(session)).toBe(false);
    });
  });

  describe('priority and edge cases', () => {
    it('should prioritize schedule_id over other detection methods', () => {
      const session = createTestSession('20250820_143000', {
        schedule_id: 'scheduler-id',
        description: '20250820_143000'
      });
      expect(isSchedulerSession(session)).toBe(true);
    });

    it('should prioritize schedule_id even with empty description', () => {
      const session = createTestSession('regular-session', {
        schedule_id: 'scheduler-id',
        description: ''
      });
      expect(isSchedulerSession(session)).toBe(true);
    });

    it('should handle complex scenarios correctly', () => {
      const schedulerSessions = [
        createTestSession('abc123', { schedule_id: 'sched-1', description: 'Any description' }),
        createTestSession('def456', { schedule_id: null, description: '' }),
        createTestSession('20250820_143000', { schedule_id: null, description: '20250820_143000' })
      ];

      schedulerSessions.forEach((session) => {
        expect(isSchedulerSession(session)).toBe(true);
      });
    });

    it('should correctly identify non-scheduler sessions', () => {
      const userSessions = [
        createTestSession('user-session-1', { schedule_id: null, description: 'My chat about AI' }),
        createTestSession('user-session-2', { schedule_id: null, description: 'Project discussion' }),
        createTestSession('20250820_143000', { schedule_id: null, description: 'Different description' })
      ];

      userSessions.forEach((session) => {
        expect(isSchedulerSession(session)).toBe(false);
      });
    });
  });
});
