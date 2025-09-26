import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useAlerts } from '../useAlerts';
import { AlertType } from '../types';

describe('useAlerts', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('Initial State', () => {
    it('should start with empty alerts array', () => {
      const { result } = renderHook(() => useAlerts());

      expect(result.current.alerts).toEqual([]);
      expect(typeof result.current.addAlert).toBe('function');
      expect(typeof result.current.clearAlerts).toBe('function');
    });
  });

  describe('Adding Alerts', () => {
    it('should add a single alert', () => {
      const { result } = renderHook(() => useAlerts());

      const newAlert = {
        type: AlertType.Info,
        message: 'Test alert',
      };

      act(() => {
        result.current.addAlert(newAlert);
      });

      expect(result.current.alerts).toHaveLength(1);
      expect(result.current.alerts[0]).toMatchObject(newAlert);
    });

    it('should add multiple alerts', () => {
      const { result } = renderHook(() => useAlerts());

      const alert1 = { type: AlertType.Info, message: 'First alert' };
      const alert2 = { type: AlertType.Warning, message: 'Second alert' };
      const alert3 = { type: AlertType.Error, message: 'Third alert' };

      act(() => {
        result.current.addAlert(alert1);
        result.current.addAlert(alert2);
        result.current.addAlert(alert3);
      });

      expect(result.current.alerts).toHaveLength(3);
      expect(result.current.alerts[0]).toMatchObject(alert1);
      expect(result.current.alerts[1]).toMatchObject(alert2);
      expect(result.current.alerts[2]).toMatchObject(alert3);
    });

    it('should add alerts with all optional properties', () => {
      const { result } = renderHook(() => useAlerts());

      const complexAlert = {
        type: AlertType.Info,
        message: 'Complex alert',
        progress: { current: 50, total: 100 },
        showCompactButton: true,
        onCompact: vi.fn(),
        compactIcon: <span>Icon</span>,
        autoShow: true,
      };

      act(() => {
        result.current.addAlert(complexAlert);
      });

      expect(result.current.alerts).toHaveLength(1);
      expect(result.current.alerts[0]).toMatchObject(complexAlert);
    });
  });

  describe('Clearing Alerts', () => {
    it('should clear all alerts', () => {
      const { result } = renderHook(() => useAlerts());

      // Add some alerts first
      act(() => {
        result.current.addAlert({ type: AlertType.Info, message: 'Alert 1' });
        result.current.addAlert({ type: AlertType.Warning, message: 'Alert 2' });
        result.current.addAlert({ type: AlertType.Error, message: 'Alert 3' });
      });

      expect(result.current.alerts).toHaveLength(3);

      // Clear all alerts
      act(() => {
        result.current.clearAlerts();
      });

      expect(result.current.alerts).toHaveLength(0);
      expect(result.current.alerts).toEqual([]);
    });

    it('should handle clearing when no alerts exist', () => {
      const { result } = renderHook(() => useAlerts());

      expect(result.current.alerts).toHaveLength(0);

      // Should not throw error
      act(() => {
        result.current.clearAlerts();
      });

      expect(result.current.alerts).toHaveLength(0);
    });
  });

  describe('Alert Management Patterns', () => {
    it('should handle rapid add and clear operations', () => {
      const { result } = renderHook(() => useAlerts());

      // Rapid operations
      act(() => {
        result.current.addAlert({ type: AlertType.Info, message: 'Alert 1' });
        result.current.addAlert({ type: AlertType.Warning, message: 'Alert 2' });
        result.current.clearAlerts();
        result.current.addAlert({ type: AlertType.Error, message: 'Alert 3' });
      });

      expect(result.current.alerts).toHaveLength(1);
      expect(result.current.alerts[0].message).toBe('Alert 3');
    });

    it('should maintain alert order', () => {
      const { result } = renderHook(() => useAlerts());

      const alerts = [
        { type: AlertType.Info, message: 'First' },
        { type: AlertType.Warning, message: 'Second' },
        { type: AlertType.Error, message: 'Third' },
        { type: AlertType.Info, message: 'Fourth' },
      ];

      act(() => {
        alerts.forEach((alert) => result.current.addAlert(alert));
      });

      expect(result.current.alerts).toHaveLength(4);
      alerts.forEach((alert, index) => {
        expect(result.current.alerts[index].message).toBe(alert.message);
      });
    });

    it('should handle duplicate alerts', () => {
      const { result } = renderHook(() => useAlerts());

      const duplicateAlert = { type: AlertType.Info, message: 'Duplicate alert' };

      act(() => {
        result.current.addAlert(duplicateAlert);
        result.current.addAlert(duplicateAlert);
        result.current.addAlert(duplicateAlert);
      });

      // Should allow duplicates
      expect(result.current.alerts).toHaveLength(3);
      result.current.alerts.forEach((alert) => {
        expect(alert.message).toBe('Duplicate alert');
      });
    });
  });

  describe('Alert Types', () => {
    it('should handle all alert types', () => {
      const { result } = renderHook(() => useAlerts());

      const alertTypes = [
        { type: AlertType.Info, message: 'Info alert' },
        { type: AlertType.Warning, message: 'Warning alert' },
        { type: AlertType.Error, message: 'Error alert' },
      ];

      act(() => {
        alertTypes.forEach((alert) => result.current.addAlert(alert));
      });

      expect(result.current.alerts).toHaveLength(3);
      expect(result.current.alerts[0].type).toBe(AlertType.Info);
      expect(result.current.alerts[1].type).toBe(AlertType.Warning);
      expect(result.current.alerts[2].type).toBe(AlertType.Error);
    });
  });

  describe('Progress Alerts', () => {
    it('should handle alerts with progress', () => {
      const { result } = renderHook(() => useAlerts());

      const progressAlert = {
        type: AlertType.Info,
        message: 'Loading...',
        progress: { current: 25, total: 100 },
      };

      act(() => {
        result.current.addAlert(progressAlert);
      });

      expect(result.current.alerts[0].progress).toEqual({ current: 25, total: 100 });
    });

    it('should handle progress updates by replacing alerts', () => {
      const { result } = renderHook(() => useAlerts());

      // Add initial progress alert
      act(() => {
        result.current.addAlert({
          type: AlertType.Info,
          message: 'Loading...',
          progress: { current: 25, total: 100 },
        });
      });

      expect(result.current.alerts[0].progress?.current).toBe(25);

      // Clear and add updated progress
      act(() => {
        result.current.clearAlerts();
        result.current.addAlert({
          type: AlertType.Info,
          message: 'Loading...',
          progress: { current: 75, total: 100 },
        });
      });

      expect(result.current.alerts).toHaveLength(1);
      expect(result.current.alerts[0].progress?.current).toBe(75);
    });
  });

  describe('Compact Button Alerts', () => {
    it('should handle alerts with compact functionality', () => {
      const { result } = renderHook(() => useAlerts());

      const mockOnCompact = vi.fn();
      const compactAlert = {
        type: AlertType.Info,
        message: 'Context window full',
        showCompactButton: true,
        onCompact: mockOnCompact,
        compactIcon: <span>📦</span>,
      };

      act(() => {
        result.current.addAlert(compactAlert);
      });

      const alert = result.current.alerts[0];
      expect(alert.showCompactButton).toBe(true);
      expect(alert.onCompact).toBe(mockOnCompact);
      expect(alert.compactIcon).toBeDefined();
    });
  });

  describe('Auto-show Alerts', () => {
    it('should handle autoShow property', () => {
      const { result } = renderHook(() => useAlerts());

      const autoShowAlert = {
        type: AlertType.Error,
        message: 'Critical error',
        autoShow: true,
      };

      act(() => {
        result.current.addAlert(autoShowAlert);
      });

      expect(result.current.alerts[0].autoShow).toBe(true);
    });

    it('should handle alerts without autoShow property', () => {
      const { result } = renderHook(() => useAlerts());

      const regularAlert = {
        type: AlertType.Info,
        message: 'Regular alert',
      };

      act(() => {
        result.current.addAlert(regularAlert);
      });

      expect(result.current.alerts[0].autoShow).toBeUndefined();
    });
  });

  describe('Edge Cases', () => {
    it('should handle empty message', () => {
      const { result } = renderHook(() => useAlerts());

      act(() => {
        result.current.addAlert({
          type: AlertType.Info,
          message: '',
        });
      });

      expect(result.current.alerts).toHaveLength(1);
      expect(result.current.alerts[0].message).toBe('');
    });

    it('should handle very long messages', () => {
      const { result } = renderHook(() => useAlerts());

      const longMessage = 'A'.repeat(1000);

      act(() => {
        result.current.addAlert({
          type: AlertType.Info,
          message: longMessage,
        });
      });

      expect(result.current.alerts[0].message).toBe(longMessage);
    });

    it('should handle special characters in messages', () => {
      const { result } = renderHook(() => useAlerts());

      const specialMessage = '🚨 Alert with émojis and spëcial chars! @#$%^&*()';

      act(() => {
        result.current.addAlert({
          type: AlertType.Warning,
          message: specialMessage,
        });
      });

      expect(result.current.alerts[0].message).toBe(specialMessage);
    });
  });
});
