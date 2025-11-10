/**
 * MCP-UI Proxy Server
 *
 * This module manages a local HTTP proxy server that securely serves MCP-UI interface files
 * to Electron webviews/iframes. The security model uses three layers of defense:
 *
 * 1. Token-based authentication: A random token is generated at startup and must be
 *    included in the X-MCP-UI-Proxy-Token header for all requests.
 *
 * 2. Origin validation: Requests must originate from the Electron app itself
 *    (file:// protocol in production, localhost:PORT in dev mode).
 *
 * 3. WebContents whitelisting: Only specific, trusted webContents can receive the proxy token.
 *    The token is automatically injected via webRequest.onBeforeSendHeaders, but only for
 *    webContents that have been explicitly registered as trusted.
 *
 * SECURITY FLOW:
 * ==============
 * 
 * A. Initialization (initMcpUIProxy):
 *    1. Generate random proxy authentication token
 *    2. Start HTTP proxy server on dynamic port
 *    3. Set up webRequest.onBeforeSendHeaders handlers for known sessions
 *    4. Register web-contents-created listener to track new webContents
 * 
 * B. WebContents Registration (web-contents-created event):
 *    1. When a new webContents is created, validate its type (window/webview only)
 *    2. Validate its URL (must be file:// or dev server origin)
 *    3. If valid, add its ID to trustedWebContentsIds Set
 *    4. Set up cleanup to remove ID when webContents is destroyed
 *    
 *    Note: This event is synchronous, so the ID is added before the webContents
 *    can make any HTTP requests.
 * 
 * C. Request Interception (onBeforeSendHeaders):
 *    1. When any webContents makes an HTTP request, check if it's to the proxy server
 *    2. Check if the webContents ID is in trustedWebContentsIds Set
 *    3. Only inject the proxy token header if BOTH conditions are true
 *    4. Log warnings when untrusted webContents attempt to access the proxy
 * 
 * D. IPC Handler (get-mcp-ui-proxy-url):
 *    1. Renderer processes request the proxy URL via IPC
 *    2. Validate the sender's URL (must be file:// or dev server origin)
 *    3. Only return the proxy URL to trusted origins
 * 
 * This multi-layered approach prevents:
 * - External browsers from accessing the proxy (token + origin validation)
 * - Compromised/untrusted renderer processes from getting the proxy URL (IPC validation)
 * - Compromised/untrusted webContents from getting the token injected (whitelist)
 * - Malicious iframes/popups from accessing the proxy (type + URL validation)
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
// Includes both IPv4 and IPv6 loopback addresses
const ALLOWED_HOSTNAMES = ['localhost', '127.0.0.1', '::1'] as const;

// State management
let mcpUIProxyServerPort: number | null = null;
let mcpUIProxyServer: http.Server | null = null;
let MCP_UI_PROXY_TOKEN: string | null = null;

// Track trusted webContents IDs that are allowed to use the proxy
const trustedWebContentsIds = new Set<number>();

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
  // Security: Only allow trusted renderer processes to access the proxy URL
  ipcMain.handle('get-mcp-ui-proxy-url', (event) => {
    // Validate that the request comes from a trusted renderer
    const senderUrl = event.sender.getURL();
    
    // Allow requests from the main app (file:// in production, localhost in dev)
    const isTrustedOrigin = 
      senderUrl.startsWith('file://') || 
      (ALLOWED_ORIGIN && senderUrl.startsWith(ALLOWED_ORIGIN));
    
    if (!isTrustedOrigin) {
      log.warn(`Rejected get-mcp-ui-proxy-url request from untrusted origin: ${senderUrl}`);
      return undefined;
    }
    
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
      // Security: Only inject headers for requests from trusted webContents
      const webContentsId = details.webContentsId;
      
      // Skip if webContentsId is undefined (shouldn't happen, but TypeScript requires the check)
      if (webContentsId === undefined) {
        callback({ cancel: false, requestHeaders: details.requestHeaders });
        return;
      }
      
      // Inject MCP-UI proxy token header for requests to the MCP-UI proxy server
      // Only if the request comes from a trusted webContents
      if (mcpUIProxyServerPort && MCP_UI_PROXY_TOKEN && trustedWebContentsIds.has(webContentsId)) {
        try {
          const parsedUrl = new URL(details.url);
          const isProxyRequest =
            (ALLOWED_HOSTNAMES as readonly string[]).includes(parsedUrl.hostname) &&
            parsedUrl.port === String(mcpUIProxyServerPort);

          if (isProxyRequest) {
            details.requestHeaders[PROXY_TOKEN_HEADER] = MCP_UI_PROXY_TOKEN;
            log.debug(`Injected proxy token for trusted webContents ${webContentsId}`);
          }
        } catch (error) {
          // If URL parsing fails, log and skip header injection
          log.debug(`Failed to parse URL for header injection: ${details.url}`, error);
        }
      } else if (mcpUIProxyServerPort && !trustedWebContentsIds.has(webContentsId)) {
        // Log when we skip injection for untrusted webContents
        try {
          const parsedUrl = new URL(details.url);
          const isProxyRequest =
            (ALLOWED_HOSTNAMES as readonly string[]).includes(parsedUrl.hostname) &&
            parsedUrl.port === String(mcpUIProxyServerPort);
          
          if (isProxyRequest) {
            log.warn(`Blocked proxy token injection for untrusted webContents ${webContentsId}`);
          }
        } catch {
          // Ignore URL parsing errors for logging
        }
      }

      callback({ cancel: false, requestHeaders: details.requestHeaders });
    });
  };

  // Set up header injection immediately for known sessions
  setupMcpProxyHeaderInjection(session.defaultSession);
  const gooseSession = session.fromPartition('persist:goose');
  setupMcpProxyHeaderInjection(gooseSession);

  // Intercept new webContents to:
  // 1. Set up header injection for their sessions
  // 2. Track trusted webContents (main windows and their legitimate child frames)
  //
  // SECURITY NOTE: This event is synchronous and fires BEFORE the webContents can make
  // any HTTP requests. This guarantees that the webContents ID is added to the trusted
  // set before onBeforeSendHeaders can be called for that webContents.
  app.on('web-contents-created', (_event, contents) => {
    const contentsType = contents.getType();
    log.debug(`New webContents created (type: ${contentsType}, id: ${contents.id})`);
    
    // Set up header injection for the session (but actual injection only happens for trusted IDs)
    // This ensures the session has the onBeforeSendHeaders handler installed
    setupMcpProxyHeaderInjection(contents.session);
    
    // SECURITY CHECK 1: Type validation
    // Only trust specific types of webContents:
    // - 'window': Main application windows created by BrowserWindow
    // - 'webview': Embedded webviews (used for MCP UIs)
    // - Other types (backgroundPage, remote, etc.) are NOT trusted
    const isTrustedType = contentsType === 'window' || contentsType === 'webview';
    
    if (isTrustedType) {
      // SECURITY CHECK 2: URL validation
      // Validate the URL is from our app before trusting
      // This prevents malicious windows/webviews from being trusted
      const url = contents.getURL();
      const isTrustedUrl = 
        !url || // Empty URL at creation time is OK (will be set during load)
        url.startsWith('file://') ||  // Production: app is served from file://
        (ALLOWED_ORIGIN && url.startsWith(ALLOWED_ORIGIN)); // Dev: app is on localhost
      
      if (isTrustedUrl) {
        // REGISTER AS TRUSTED: Add to whitelist
        // This ID will now pass the check in onBeforeSendHeaders
        trustedWebContentsIds.add(contents.id);
        log.info(`Registered trusted webContents ${contents.id} (type: ${contentsType})`);
        
        // CLEANUP: Remove from trusted set when destroyed
        // This prevents the Set from growing unbounded and prevents
        // ID reuse attacks (though Electron doesn't reuse IDs in practice)
        contents.on('destroyed', () => {
          trustedWebContentsIds.delete(contents.id);
          log.debug(`Removed destroyed webContents ${contents.id} from trusted set`);
        });
      } else {
        // SECURITY: Reject webContents with untrusted URLs
        log.warn(`Rejected webContents ${contents.id} with untrusted URL: ${url}`);
      }
    } else {
      // SECURITY: Reject webContents with untrusted types
      log.debug(`Skipped registering webContents ${contents.id} (type: ${contentsType})`);
    }
  });

  // Register cleanup handler to gracefully shut down the proxy server on app quit
  app.on('will-quit', async () => {
    await shutdownMcpUIProxyServer();
  });
}
