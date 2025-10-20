/**
 * Debug Logger Utility
 *
 * Provides structured logging for development that's automatically removed in production builds.
 * Uses process.env.NODE_ENV for tree-shaking - bundlers will remove all debug code in production.
 *
 * Optionally integrates with electron-log for persistent file logging.
 */

const isDevelopment = process.env.NODE_ENV === 'development';

// Optional electron logging integration via window.electron.logInfo
let useElectronLog = false;

/**
 * Enable electron logging for persistent file logging via window.electron.logInfo
 * This sends logs to the main process which writes them to disk using electron-log.
 * @param enable - Whether to enable electron logging (default: false)
 */
export function setElectronLogEnabled(enable: boolean) {
  useElectronLog = enable;
}

/**
 * Unified logging output function
 * Logs to console and optionally to electron main process
 */
function logOutput(consoleMethod: 'log' | 'error' | 'warn', message: string, details?: unknown) {
  // Log to console
  if (consoleMethod === 'log') {
    console.log(message, details || '');
  } else if (consoleMethod === 'error') {
    console.error(message, details);
  } else if (consoleMethod === 'warn') {
    console.warn(message, details || '');
  }

  // Optionally log to electron
  if (useElectronLog) {
    try {
      const logMessage = details ? `${message} ${JSON.stringify(details)}` : message;
      window.electron?.logInfo(logMessage);
    } catch {
      // Silently fail if window.electron is not available
    }
  }
}

/**
 * Creates a namespaced logger with categorized logging methods
 * @param namespace - The namespace for this logger (e.g., 'useChatStream', 'BaseChat2')
 * @returns Logger object with categorized methods
 */
export function createDebugLogger(namespace: string) {
  const createLogger = (category: string) => {
    return (action: string, details?: Record<string, unknown>) => {
      if (!isDevelopment) return;
      const message = `[${namespace}:${category}] ${action}`;
      logOutput('log', message, details);
    };
  };

  const createGroupLogger = (emoji: string, title: string) => {
    return (details?: Record<string, unknown>) => {
      if (!isDevelopment) return;
      console.group(`${emoji} [${namespace}] ${title}`);
      if (details) {
        Object.entries(details).forEach(([key, value]) => {
          console.log(`${key}:`, value);
        });
      }
    };
  };

  return {
    // Generic logging
    log: (message: string, details?: Record<string, unknown>) => {
      if (!isDevelopment) return;
      const fullMessage = `[${namespace}] ${message}`;
      logOutput('log', fullMessage, details);
    },

    // Error logging (always enabled, even in production)
    error: (context: string, error: unknown) => {
      const fullMessage = `[${namespace}:error] ${context}`;
      logOutput('error', fullMessage, error);
    },

    // Warn logging (always enabled, even in production)
    warn: (message: string, details?: Record<string, unknown>) => {
      const fullMessage = `[${namespace}:warn] ${message}`;
      logOutput('warn', fullMessage, details);
    },

    // Category-specific loggers
    session: createLogger('session'),
    messages: createLogger('messages'),
    stream: createLogger('stream'),
    state: createLogger('state'),
    cache: createLogger('cache'),

    // Grouped logging for complex operations
    group: createGroupLogger,
    groupEnd: () => {
      if (!isDevelopment) return;
      console.groupEnd();
    },

    // Special formatted loggers with emojis
    cacheCheck: (details: Record<string, unknown>) => {
      if (!isDevelopment) return;
      console.group(`🔍 [${namespace}] Cache Check`);
      Object.entries(details).forEach(([key, value]) => {
        console.log(`${key}:`, value);
      });
      logOutput('log', `🔍 [${namespace}] Cache Check`, details);
    },

    cacheHit: (details: Record<string, unknown>) => {
      if (!isDevelopment) return;
      console.log(`✅ [${namespace}] CACHE HIT - Using cached data for instant display!`);
      console.log('Cache Details:', details);
      console.groupEnd();
      logOutput('log', `✅ [${namespace}] CACHE HIT`, details);
    },

    cacheMiss: (reason: string) => {
      if (!isDevelopment) return;
      console.log(`❌ [${namespace}] CACHE MISS - ${reason}`);
      console.groupEnd();
      logOutput('log', `❌ [${namespace}] CACHE MISS - ${reason}`);
    },

    newChat: (message: string) => {
      if (!isDevelopment) return;
      const fullMessage = `🆕 [${namespace}] ${message}`;
      logOutput('log', fullMessage);
    },

    tracking: (message: string, id?: string) => {
      if (!isDevelopment) return;
      const idStr = id ? `: ${id.slice(0, 8)}` : '';
      const fullMessage = `📌 [${namespace}] ${message}${idStr}`;
      logOutput('log', fullMessage);
    },
  };
}

/**
 * Helper to create a session-aware logger that automatically truncates session IDs
 */
export function createSessionLogger(namespace: string) {
  const logger = createDebugLogger(namespace);

  return {
    ...logger,
    session: (action: string, sessionId: string, details?: Record<string, unknown>) => {
      if (!isDevelopment) return;
      logger.session(action, {
        sessionId: sessionId.slice(0, 8),
        ...details,
      });
    },
  };
}

/**
 * Helper to create a messages logger that includes count
 */
export function createMessagesLogger(namespace: string) {
  const logger = createDebugLogger(namespace);

  return {
    ...logger,
    messages: (action: string, count: number, details?: Record<string, unknown>) => {
      if (!isDevelopment) return;
      logger.messages(action, {
        count,
        ...details,
      });
    },
  };
}

// Export a default logger for quick use
export const debugLog = createDebugLogger('app');
