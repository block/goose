/**
 * MCP-UI Proxy Server
 *
 * This module manages a local HTTP proxy server that securely serves MCP-UI interface files
 * to Electron webviews/iframes. The security model uses two layers of defense:
 *
 * 1. Token-based authentication: A random token is generated at startup and must be
 *    included in the X-MCP-UI-Proxy-Token header for all requests.
 *
 * 2. Origin validation: Requests must originate from the Electron app itself
 *    (file:// protocol in production, localhost:PORT in dev mode).
 *
 * This prevents external browsers or malicious websites from accessing the proxy server,
 * even if they discover the port number.
 */

import * as crypto from 'crypto';
import type { Session } from 'electron';
import { app, ipcMain, session } from 'electron';
import express from 'express';
import fsSync from 'node:fs';
import http from 'node:http';
import path from 'node:path';
import log from './utils/logger';

// Security constants
const TOKEN_BYTE_LENGTH = 32;
const TOKEN_PREFIX = 'mcp-ui-proxy';
const PROXY_TOKEN_HEADER = 'x-mcp-ui-proxy-token';
const PROXY_HTML_PATH = '/mcp-ui-proxy.html';

// Server configuration
const PROXY_SERVER_HOST = 'localhost';
const PROXY_SERVER_PORT = 0; // 0 = OS assigns available port

// Allowed hostnames for proxy requests
const ALLOWED_HOSTNAMES = ['localhost', '127.0.0.1'] as const;

// State management
let mcpUIProxyServerPort: number | null = null;
let mcpUIProxyServer: http.Server | null = null;
let MCP_UI_PROXY_TOKEN: string | null = null;

// Type definitions
interface SecurityCheckResult {
  isValid: boolean;
  reason?: string;
}

/**
 * Validates the MCP-UI proxy token from request headers
 */
function validateProxyToken(token: string | undefined): SecurityCheckResult {
  if (token !== MCP_UI_PROXY_TOKEN) {
    return {
      isValid: false,
      reason: 'Invalid or missing proxy token',
    };
  }
  return { isValid: true };
}

/**
 * Validates that the request originates from the Electron app
 * @param origin - The origin or referer header from the request
 * @param allowedOrigin - The expected origin (null in production for file:// protocol)
 */
function validateOrigin(
  origin: string | undefined,
  allowedOrigin: string | null
): SecurityCheckResult {
  let isElectronRequest = false;

  if (allowedOrigin) {
    // Dev mode: require exact localhost:port match
    isElectronRequest = origin?.startsWith(allowedOrigin) || false;
  } else {
    // Production mode: accept file:// protocol or missing origin
    // (iframes in file:// context often don't send origin/referer)
    isElectronRequest = !origin || origin.startsWith('file://');
  }

  if (!isElectronRequest) {
    return {
      isValid: false,
      reason: `Invalid origin. Got: ${origin || 'none'}, Expected: ${allowedOrigin || 'file:// or no origin'}`,
    };
  }

  return { isValid: true };
}

/**
 * Determines the correct path to static files based on environment
 * @returns Absolute path to the static directory
 */
function getStaticPath(): string {
  // In production: extraResources are in process.resourcesPath
  // In dev: relative to the build directory
  return app.isPackaged
    ? path.join(process.resourcesPath, 'static')
    : path.join(__dirname, '../../static');
}

/**
 * Gracefully shuts down the MCP-UI proxy server
 */
async function shutdownMcpUIProxyServer(): Promise<void> {
  if (!mcpUIProxyServer) {
    return;
  }

  return new Promise<void>((resolve) => {
    mcpUIProxyServer!.close(() => {
      log.info('MCP UI Proxy server closed');
      mcpUIProxyServer = null;
      mcpUIProxyServerPort = null;
      MCP_UI_PROXY_TOKEN = null;
      resolve();
    });
  });
}

/**
 * Initializes the MCP-UI proxy server and security infrastructure
 * @param devUrl - The development server URL (undefined or empty string in production)
 */
export async function initMcpUIProxy(devUrl: string | undefined): Promise<void> {
  // Compute allowed origin for dev mode (null in production)
  const ALLOWED_ORIGIN = devUrl ? new URL(devUrl).origin : null;

  // Generate secure random token for proxy authentication
  MCP_UI_PROXY_TOKEN = `${TOKEN_PREFIX}:${crypto.randomBytes(TOKEN_BYTE_LENGTH).toString('hex')}`;

  // IPC handler to provide the proxy URL to renderer processes
  ipcMain.handle('get-mcp-ui-proxy-url', () => {
    if (mcpUIProxyServerPort) {
      return `http://${PROXY_SERVER_HOST}:${mcpUIProxyServerPort}${PROXY_HTML_PATH}`;
    }
    return undefined;
  });

  /**
   * Starts the MCP-UI proxy HTTP server on a dynamic port
   * @returns Promise resolving to the assigned port number
   * @throws Error if server fails to start or bind to a port
   */
  async function startMcpUIProxyServer(): Promise<number> {
    return new Promise((resolve, reject) => {
      const expressApp = express();

      // Get the appropriate static file directory
      const staticPath = getStaticPath();

      // Verify static directory exists and log contents for debugging
      if (fsSync.existsSync(staticPath)) {
        const files = fsSync.readdirSync(staticPath);
        log.info(`MCP UI Proxy: static dir contents = ${files.join(', ')}`);
      } else {
        log.error(`MCP UI Proxy: static directory not found at ${staticPath}`);
      }

      // Security middleware: validate token and origin on all requests
      expressApp.use((req, res, next) => {
        // Validate token (required for all requests)
        const token = req.headers[PROXY_TOKEN_HEADER] as string | undefined;
        const tokenCheck = validateProxyToken(token);
        if (!tokenCheck.isValid) {
          log.warn(`MCP-UI Proxy unauthorized: ${tokenCheck.reason}. IP: ${req.ip}`);
          res.status(403).send('Forbidden');
          return;
        }

        // Validate origin (defense in depth - ensure request is from Electron app)
        const origin = req.headers.origin || req.headers.referer;
        const originCheck = validateOrigin(origin, ALLOWED_ORIGIN);
        if (!originCheck.isValid) {
          log.warn(`MCP-UI Proxy unauthorized: ${originCheck.reason}. IP: ${req.ip}`);
          res.status(403).send('Forbidden');
          return;
        }

        next();
      });

      // Serve static files from the static directory
      expressApp.use(express.static(staticPath));

      // Create HTTP server
      mcpUIProxyServer = http.createServer(expressApp);

      // Listen on a dynamic port (0 = let the OS choose an available port)
      mcpUIProxyServer.listen(PROXY_SERVER_PORT, PROXY_SERVER_HOST, () => {
        const address = mcpUIProxyServer?.address();
        if (address && typeof address === 'object') {
          mcpUIProxyServerPort = address.port;
          log.info(`MCP UI Proxy server started on ${PROXY_SERVER_HOST}:${mcpUIProxyServerPort}`);
          resolve(mcpUIProxyServerPort);
        } else {
          reject(new Error('Failed to get server address'));
        }
      });

      mcpUIProxyServer.on('error', (error) => {
        log.error('MCP UI Proxy server error:', error);
        reject(error);
      });
    });
  }

  // Start MCP-UI proxy server
  try {
    await startMcpUIProxyServer();
  } catch (error) {
    log.error('Failed to start MCP-UI proxy server:', error);
  }

  // Track sessions that have been set up to avoid duplicate handlers
  const configuredSessions = new WeakSet<Session>();

  /**
   * Sets up HTTP header injection for MCP-UI proxy requests on a given session
   * Ensures the proxy token is automatically included in requests to the proxy server
   * This runs once per session to avoid duplicate handlers
   * @param sess - The Electron session to configure
   */
  const setupMcpProxyHeaderInjection = (sess: Session): void => {
    // Skip if we've already configured this session
    if (configuredSessions.has(sess)) {
      return;
    }
    configuredSessions.add(sess);

    log.debug(`Setting up MCP-UI proxy header injection for session`);

    sess.webRequest.onBeforeSendHeaders((details, callback) => {
      // Inject MCP-UI proxy token header for requests to the MCP-UI proxy server
      if (mcpUIProxyServerPort && MCP_UI_PROXY_TOKEN) {
        try {
          const parsedUrl = new URL(details.url);
          const isProxyRequest =
            (ALLOWED_HOSTNAMES as readonly string[]).includes(parsedUrl.hostname) &&
            parsedUrl.port === String(mcpUIProxyServerPort);

          if (isProxyRequest) {
            details.requestHeaders[PROXY_TOKEN_HEADER] = MCP_UI_PROXY_TOKEN;
          }
        } catch (error) {
          // If URL parsing fails, log and skip header injection
          log.debug(`Failed to parse URL for header injection: ${details.url}`, error);
        }
      }

      callback({ cancel: false, requestHeaders: details.requestHeaders });
    });
  };

  // Set up header injection immediately for known sessions
  setupMcpProxyHeaderInjection(session.defaultSession);
  const gooseSession = session.fromPartition('persist:goose');
  setupMcpProxyHeaderInjection(gooseSession);

  // Intercept all new webContents (including main window and iframes) to set up header injection
  // This ensures that any dynamically created webviews or iframes also get the proxy token
  app.on('web-contents-created', (_event, contents) => {
    log.debug(
      `New webContents created (type: ${contents.getType()}), setting up MCP-UI proxy headers`
    );
    setupMcpProxyHeaderInjection(contents.session);
  });

  // Register cleanup handler to gracefully shut down the proxy server on app quit
  app.on('will-quit', async () => {
    await shutdownMcpUIProxyServer();
  });
}
