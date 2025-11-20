/**
 * Utilities for session management
 */

/**
 * Generate a unique session ID
 * Format: YYYYMMDD_HHMMSS_RANDOM
 */
export function generateSessionId(): string {
  const now = new Date();
  const dateStr = now.toISOString().slice(0, 10).replace(/-/g, ''); // YYYYMMDD
  const timeStr = now.toTimeString().slice(0, 8).replace(/:/g, ''); // HHMMSS
  const randomStr = Math.random().toString(36).substr(2, 6); // Random 6 chars
  
  return `${dateStr}_${timeStr}_${randomStr}`;
}

/**
 * Check if a session ID is valid format
 */
export function isValidSessionId(sessionId: string): boolean {
  // Check for various valid formats
  const patterns = [
    /^\d{8}_\d{6}_[a-z0-9]{6}$/, // New format: YYYYMMDD_HHMMSS_RANDOM
    /^[a-f0-9-]{36}$/, // UUID format
    /^!\w+:\w+\.\w+$/, // Matrix room ID format
    /^\w{8,}$/, // Generic alphanumeric ID (8+ chars)
  ];
  
  return patterns.some(pattern => pattern.test(sessionId));
}

/**
 * Extract timestamp from session ID if possible
 */
export function getSessionTimestamp(sessionId: string): Date | null {
  const match = sessionId.match(/^(\d{8})_(\d{6})_/);
  if (!match) return null;
  
  const [, dateStr, timeStr] = match;
  const year = parseInt(dateStr.slice(0, 4));
  const month = parseInt(dateStr.slice(4, 6)) - 1; // Month is 0-indexed
  const day = parseInt(dateStr.slice(6, 8));
  const hour = parseInt(timeStr.slice(0, 2));
  const minute = parseInt(timeStr.slice(2, 4));
  const second = parseInt(timeStr.slice(4, 6));
  
  return new Date(year, month, day, hour, minute, second);
}

/**
 * Generate a human-readable session title
 */
export function generateSessionTitle(sessionId: string, fallback = 'Chat Session'): string {
  const timestamp = getSessionTimestamp(sessionId);
  if (timestamp) {
    const timeStr = timestamp.toLocaleTimeString([], { 
      hour: '2-digit', 
      minute: '2-digit' 
    });
    return `Chat ${timeStr}`;
  }
  
  // For other session ID formats, use a generic title
  if (sessionId.startsWith('!')) {
    return 'Matrix Chat';
  }
  
  return fallback;
}
