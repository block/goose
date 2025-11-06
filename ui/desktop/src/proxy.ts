import type { Session } from 'electron';
import { ipcMain, app, session } from 'electron';
import express from 'express';
import http from 'node:http';
import * as crypto from 'crypto';
import path from 'node:path';
import fsSync from 'node:fs';
import log from './utils/logger';

// HTTP server for serving MCP-UI proxy files
let mcpUIProxyServerPort: number | null = null;
let mcpUIProxyServer: http.Server | null = null;
// Random token to secure the MCP-UI proxy server from access by external browsers
let MCP_UI_PROXY_TOKEN: string | null = null;

export async function initMcpUIProxy(devUrl: string) {
  // Compute allowed origin for dev mode (null in production)
  const ALLOWED_ORIGIN = devUrl ? new URL(devUrl).origin : null;

  // Generate token
  MCP_UI_PROXY_TOKEN = `mcp-ui-proxy:${crypto.randomBytes(32).toString('hex')}`;

  ipcMain.handle('get-mcp-ui-proxy-url', () => {
    if (mcpUIProxyServerPort) {
      return `http://localhost:${mcpUIProxyServerPort}/mcp-ui-proxy.html`;
    }
    return undefined;
  });

  async function startMcpUIProxyServer(): Promise<number> {
    return new Promise((resolve, reject) => {
      const expressApp = express();

      // Determine static path based on environment
      // In dev: __dirname is like .../ui/desktop/.vite/build, use relative path
      // In prod: extraResources are in process.resourcesPath (e.g., Goose.app/Contents/Resources/)
      const staticPath = app.isPackaged
        ? path.join(process.resourcesPath, 'static')
        : path.join(__dirname, '../../static');

      if (fsSync.existsSync(staticPath)) {
        const files = fsSync.readdirSync(staticPath);
        log.info(`MCP UI Proxy: static dir contents = ${files.join(', ')}`);
      } else {
        log.error(`MCP UI Proxy: static directory not found at ${staticPath}`);
      }

      // Security middleware: validate token and origin on all requests
      expressApp.use((req, res, next) => {
        // Check 1: Validate token via header
        const token = req.headers['x-mcp-ui-proxy-token'] as string | undefined;
        if (token !== MCP_UI_PROXY_TOKEN) {
          log.warn(`Unauthorized access attempt to MCP-UI proxy from ${req.ip} - invalid token`);
          res.status(403).send('Forbidden');
          return;
        }

        // Check 2: Validate origin (defense in depth) - require exact Electron origin
        const origin = req.headers.origin || req.headers.referer;
        let isElectronRequest = false;

        if (ALLOWED_ORIGIN) {
          // Dev mode: check for exact localhost:port match
          isElectronRequest = origin?.startsWith(ALLOWED_ORIGIN) || false;
        } else {
          // Production mode: check for file:// protocol OR allow missing origin
          // (iframes in file:// context often don't send origin/referer headers)
          isElectronRequest = !origin || origin.startsWith('file://');
        }

        if (!isElectronRequest) {
          log.warn(
            `Unauthorized access attempt to MCP-UI proxy. Origin: ${origin || 'none'}, Expected: ${ALLOWED_ORIGIN || 'file:// or no origin'}, IP: ${req.ip}`
          );
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
      mcpUIProxyServer.listen(0, 'localhost', () => {
        const address = mcpUIProxyServer?.address();
        if (address && typeof address === 'object') {
          mcpUIProxyServerPort = address.port;
          log.info(`MCP UI Proxy server started on port ${mcpUIProxyServerPort}`);
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

  // Helper function to set up header injection for a session (only once per session)
  // Handles both Origin header (for dev mode) and MCP-UI proxy token injection
  const setupMcpProxyHeaderInjection = (sess: Session) => {
    // Skip if we've already configured this session
    if (configuredSessions.has(sess)) {
      return;
    }
    configuredSessions.add(sess);

    log.debug(`Setting up header injection for session`);

    sess.webRequest.onBeforeSendHeaders((details, callback) => {
      // Set up Origin header for all requests on default session (existing behavior for dev mode)

      // Inject MCP-UI proxy token header for requests to the MCP-UI proxy server
      if (mcpUIProxyServerPort && MCP_UI_PROXY_TOKEN) {
        try {
          const parsedUrl = new URL(details.url);
          // Accept both 'localhost' and '127.0.0.1' as hostnames
          if (
            (parsedUrl.hostname === 'localhost' || parsedUrl.hostname === '127.0.0.1') &&
            parsedUrl.port === String(mcpUIProxyServerPort)
          ) {
            // Use lowercase to match Express's normalized header names
            details.requestHeaders['x-mcp-ui-proxy-token'] = MCP_UI_PROXY_TOKEN;
          }
        } catch {
          // If URL parsing fails, do not inject the header
        }
      }

      callback({ cancel: false, requestHeaders: details.requestHeaders });
    });
  };

  // Set up header injection immediately for known sessions
  setupMcpProxyHeaderInjection(session.defaultSession);
  const gooseSession = session.fromPartition('persist:goose');
  setupMcpProxyHeaderInjection(gooseSession);

  // Intercept all new webContents (including main window and iframes) and set up MCP-UI proxy header injection
  app.on('web-contents-created', (_event, contents) => {
    log.debug(`New webContents created: ${contents.getType()}`);
    setupMcpProxyHeaderInjection(contents.session);
  });

  // Register cleanup handler for app quit
  app.on('will-quit', async () => {
    // Close MCP-UI proxy server
    if (mcpUIProxyServer) {
      const server = mcpUIProxyServer as http.Server;
      await new Promise<void>((resolve) => {
        server.close(() => {
          log.info('MCP UI Proxy server closed');
          resolve();
        });
      });
      mcpUIProxyServer = null;
      mcpUIProxyServerPort = null;
    }
  });
}
