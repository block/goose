import express from 'express';
import cors from 'cors';
import path from 'node:path';
import { Server } from 'node:http';
import { createServer } from 'net';
import { createProxyMiddleware } from 'http-proxy-middleware';
import log from './utils/logger';

declare var MAIN_WINDOW_VITE_DEV_SERVER_URL: string;
declare var MAIN_WINDOW_VITE_NAME: string;

let webServer: Server | null = null;

// Find an available port for the web server
const findAvailablePort = (): Promise<number> => {
  return new Promise((resolve, _reject) => {
    const server = createServer();

    server.listen(0, '127.0.0.1', () => {
      const { port } = server.address() as { port: number };
      server.close(() => {
        log.info(`Found available port for web server: ${port}`);
        resolve(port);
      });
    });
  });
};

export const startWebServer = async (
  goosePort: number,
  workingDir: string,
  secretKey: string
): Promise<number> => {
  const port = await findAvailablePort();
  const app = express();

  // Enable CORS for all origins (since this is localhost-only)
  app.use(
    cors({
      origin: '*',
      credentials: true,
    })
  );

  // Parse JSON bodies for POST requests
  app.use(express.json());

  // Search endpoint that triggers a chat with the query
  app.get('/search', (req, res) => {
    const query = req.query.q as string;

    if (!query) {
      res.status(400).json({ error: 'Query parameter "q" is required' });
      return;
    }

    log.info(`Search request received with query: ${query}`);

    // Generate HTML that will auto-start a chat with the query
    const searchHtml = `
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Goose - Search: ${query.substring(0, 50)}${query.length > 50 ? '...' : ''}</title>
    <script src="/goose-config.js"></script>
    <style>
      body {
        margin: 0;
        padding: 20px;
        font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Roboto', sans-serif;
        background: #1a1a1a;
        color: #ffffff;
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        min-height: 100vh;
      }
      .loading {
        text-align: center;
        margin-bottom: 20px;
      }
      .spinner {
        border: 2px solid #333;
        border-top: 2px solid #fff;
        border-radius: 50%;
        width: 40px;
        height: 40px;
        animation: spin 1s linear infinite;
        margin: 0 auto 20px;
      }
      @keyframes spin {
        0% { transform: rotate(0deg); }
        100% { transform: rotate(360deg); }
      }
      .query {
        background: #2a2a2a;
        padding: 15px;
        border-radius: 8px;
        margin: 20px 0;
        font-family: monospace;
        word-break: break-word;
      }
    </style>
</head>
<body>
    <div class="loading">
        <div class="spinner"></div>
        <h2>Starting Goose Chat...</h2>
        <div class="query">Query: "${query}"</div>
        <p>Redirecting to chat interface...</p>
    </div>
    
    <script>
        // Store the search query for the chat to pick up
        sessionStorage.setItem('autoStartQuery', ${JSON.stringify(query)});
        
        // Redirect to the main chat interface after a brief delay
        setTimeout(() => {
            window.location.href = '/';
        }, 1500);
    </script>
</body>
</html>`;

    res.setHeader('Content-Type', 'text/html');
    res.send(searchHtml);
  });

  // Serve static files
  if (MAIN_WINDOW_VITE_DEV_SERVER_URL) {
    // In development, serve config and proxy to Vite
    log.info(
      `Development mode: serving with Vite dev server at ${MAIN_WINDOW_VITE_DEV_SERVER_URL}`
    );

    // Serve the config as a separate endpoint
    app.get('/goose-config.js', (_req, res) => {
      const config = {
        GOOSE_DEFAULT_PROVIDER: process.env.GOOSE_DEFAULT_PROVIDER,
        GOOSE_DEFAULT_MODEL: process.env.GOOSE_DEFAULT_MODEL,
        GOOSE_API_HOST: 'http://127.0.0.1',
        GOOSE_PORT: goosePort,
        GOOSE_WORKING_DIR: workingDir,
        GOOSE_ALLOWLIST_WARNING: process.env.GOOSE_ALLOWLIST_WARNING === 'true',
        secretKey: secretKey,
        GOOSE_BASE_URL_SHARE: process.env.GOOSE_BASE_URL_SHARE,
        GOOSE_VERSION: process.env.GOOSE_VERSION,
      };

      const configScript = `
window.gooseConfig = ${JSON.stringify(config)};
window.appConfig = {
  get: (key) => window.gooseConfig[key],
  getAll: () => window.gooseConfig
};
// Comprehensive mock of window.electron for browser compatibility
window.electron = {
  // Core configuration
  getConfig: () => window.gooseConfig,
  platform: 'web',
  
  // App lifecycle
  reactReady: () => console.log('[Browser] reactReady called'),
  reloadApp: () => window.location.reload(),
  
  // Logging
  logInfo: (msg) => console.info('[Browser]', msg),
  
  // Window management
  createChatWindow: () => window.open(window.location.href, '_blank'),
  hideWindow: () => console.log('[Browser] Hide window (no-op)'),
  
  // Extensions
  getAllowedExtensions: () => Promise.resolve([]),
  getBinaryPath: () => Promise.resolve(''),
  
  // Event handling
  on: () => {},
  off: () => {},
  emit: () => console.log('[Browser] Event emission (no-op)'),
  
  // Power management
  startPowerSaveBlocker: () => console.log('[Browser] Power save blocker started (no-op)'),
  stopPowerSaveBlocker: () => console.log('[Browser] Power save blocker stopped (no-op)'),
  
  // Notifications
  showNotification: (title, body) => {
    console.log('[Browser] Notification:', title, body);
    if ('Notification' in window && Notification.permission === 'granted') {
      new Notification(title, { body });
    }
  },
  
  // File operations
  selectFileOrDirectory: () => {
    console.log('[Browser] File selection not available in browser');
    return Promise.resolve(null);
  },
  saveDataUrlToTemp: () => {
    console.log('[Browser] Temp file operations not available in browser');
    return Promise.resolve(null);
  },
  deleteTempFile: () => {
    console.log('[Browser] Temp file operations not available in browser');
    return Promise.resolve();
  },
  readFile: () => {
    console.log('[Browser] File reading not available in browser');
    return Promise.resolve('');
  },
  writeFile: () => {
    console.log('[Browser] File writing not available in browser');
    return Promise.resolve();
  },
  ensureDirectory: () => {
    console.log('[Browser] Directory operations not available in browser');
    return Promise.resolve();
  },
  listFiles: () => {
    console.log('[Browser] File listing not available in browser');
    return Promise.resolve([]);
  },
  directoryChooser: () => {
    console.log('[Browser] Directory chooser not available in browser');
    return Promise.resolve(null);
  },
  getPathForFile: () => {
    console.log('[Browser] Path operations not available in browser');
    return Promise.resolve('');
  },
  
  // Image/media operations
  getTempImage: () => {
    console.log('[Browser] Image operations not available in browser');
    return Promise.resolve(null);
  },
  fetchMetadata: () => {
    console.log('[Browser] Metadata fetching not available in browser');
    return Promise.resolve({});
  },
  
  // System settings
  setDockIcon: () => console.log('[Browser] Dock icon setting (no-op)'),
  setMenuBarIcon: () => console.log('[Browser] Menu bar icon setting (no-op)'),
  setQuitConfirmation: () => console.log('[Browser] Quit confirmation setting (no-op)'),
  getDockIconState: (callback) => callback && callback(false),
  getMenuBarIconState: (callback) => callback && callback(false),
  getQuitConfirmationState: (callback) => callback && callback(false),
  openNotificationsSettings: () => console.log('[Browser] Notification settings (no-op)'),
  
  // Updates
  getVersion: () => Promise.resolve('web-version'),
  checkForUpdates: () => console.log('[Browser] Update checking (no-op)'),
  downloadUpdate: () => console.log('[Browser] Update downloading (no-op)'),
  installUpdate: () => console.log('[Browser] Update installation (no-op)'),
  getUpdateState: (callback) => callback && callback({ available: false }),
  onUpdaterEvent: () => {},
  
  // Scheduler
  getSettings: () => Promise.resolve({}),
  setSchedulingEngine: () => console.log('[Browser] Scheduling engine setting (no-op)'),
  
  // Dialog
  showMessageBox: () => {
    console.log('[Browser] Message box not available in browser');
    return Promise.resolve({ response: 0 });
  },
  
  // IPC
  ipcRenderer: {
    send: () => console.warn('IPC not available in web version'),
    invoke: () => Promise.resolve(null),
    on: () => {},
    removeAllListeners: () => {}
  }
};
console.log('Goose config loaded for web version:', window.gooseConfig);
`;
      res.setHeader('Content-Type', 'application/javascript');
      res.send(configScript);
    });

    // For the root route, fetch from Vite and inject config
    app.get('/', async (_req, res) => {
      try {
        const response = await fetch(MAIN_WINDOW_VITE_DEV_SERVER_URL);
        const html = await response.text();

        // Inject config script into HTML
        const modifiedHtml = html.replace(
          '<head>',
          '<head>\n    <script src="/goose-config.js"></script>'
        );

        res.send(modifiedHtml);
      } catch (error) {
        log.error('Error fetching from Vite dev server:', error);
        res.status(500).send('Error loading development server');
      }
    });

    // Proxy all other requests to Vite dev server
    const proxy = createProxyMiddleware({
      target: MAIN_WINDOW_VITE_DEV_SERVER_URL,
      changeOrigin: true,
      ws: true, // proxy websockets for HMR
    });

    app.use('/', proxy);
  } else {
    // In production, serve the built files
    const staticPath = path.join(__dirname, `../renderer/${MAIN_WINDOW_VITE_NAME}`);
    log.info(`Serving static files from: ${staticPath}`);

    // Serve static assets
    app.use(express.static(staticPath));

    // Inject configuration into the HTML
    app.get('*', (_req, res) => {
      try {
        const indexPath = path.join(staticPath, 'index.html');
        const fs = require('fs'); // eslint-disable-line @typescript-eslint/no-var-requires
        let html = fs.readFileSync(indexPath, 'utf8');

        // Inject the configuration
        const config = {
          GOOSE_DEFAULT_PROVIDER: process.env.GOOSE_DEFAULT_PROVIDER,
          GOOSE_DEFAULT_MODEL: process.env.GOOSE_DEFAULT_MODEL,
          GOOSE_API_HOST: 'http://127.0.0.1',
          GOOSE_PORT: goosePort,
          GOOSE_WORKING_DIR: workingDir,
          GOOSE_ALLOWLIST_WARNING: process.env.GOOSE_ALLOWLIST_WARNING === 'true',
          secretKey: secretKey,
          GOOSE_BASE_URL_SHARE: process.env.GOOSE_BASE_URL_SHARE,
          GOOSE_VERSION: process.env.GOOSE_VERSION,
        };

        // Inject config before closing head tag
        const configScript = `
<script>
  window.gooseConfig = ${JSON.stringify(config)};
  window.appConfig = {
    get: (key) => window.gooseConfig[key],
    getAll: () => window.gooseConfig
  };
  // Comprehensive mock of window.electron for browser compatibility
  window.electron = {
    // Core configuration
    getConfig: () => window.gooseConfig,
    platform: 'web',
    
    // App lifecycle
    reactReady: () => console.log('[Browser] reactReady called'),
    reloadApp: () => window.location.reload(),
    
    // Logging
    logInfo: (msg) => console.info('[Browser]', msg),
    
    // Window management
    createChatWindow: () => window.open(window.location.href, '_blank'),
    hideWindow: () => console.log('[Browser] Hide window (no-op)'),
    
    // Extensions
    getAllowedExtensions: () => Promise.resolve([]),
    getBinaryPath: () => Promise.resolve(''),
    
    // Event handling
    on: () => {},
    off: () => {},
    emit: () => console.log('[Browser] Event emission (no-op)'),
    
    // Power management
    startPowerSaveBlocker: () => console.log('[Browser] Power save blocker started (no-op)'),
    stopPowerSaveBlocker: () => console.log('[Browser] Power save blocker stopped (no-op)'),
    
    // Notifications
    showNotification: (title, body) => {
      console.log('[Browser] Notification:', title, body);
      if ('Notification' in window && Notification.permission === 'granted') {
        new Notification(title, { body });
      }
    },
    
    // File operations
    selectFileOrDirectory: () => {
      console.log('[Browser] File selection not available in browser');
      return Promise.resolve(null);
    },
    saveDataUrlToTemp: () => {
      console.log('[Browser] Temp file operations not available in browser');
      return Promise.resolve(null);
    },
    deleteTempFile: () => {
      console.log('[Browser] Temp file operations not available in browser');
      return Promise.resolve();
    },
    readFile: () => {
      console.log('[Browser] File reading not available in browser');
      return Promise.resolve('');
    },
    writeFile: () => {
      console.log('[Browser] File writing not available in browser');
      return Promise.resolve();
    },
    ensureDirectory: () => {
      console.log('[Browser] Directory operations not available in browser');
      return Promise.resolve();
    },
    listFiles: () => {
      console.log('[Browser] File listing not available in browser');
      return Promise.resolve([]);
    },
    directoryChooser: () => {
      console.log('[Browser] Directory chooser not available in browser');
      return Promise.resolve(null);
    },
    getPathForFile: () => {
      console.log('[Browser] Path operations not available in browser');
      return Promise.resolve('');
    },
    
    // Image/media operations
    getTempImage: () => {
      console.log('[Browser] Image operations not available in browser');
      return Promise.resolve(null);
    },
    fetchMetadata: () => {
      console.log('[Browser] Metadata fetching not available in browser');
      return Promise.resolve({});
    },
    
    // System settings
    setDockIcon: () => console.log('[Browser] Dock icon setting (no-op)'),
    setMenuBarIcon: () => console.log('[Browser] Menu bar icon setting (no-op)'),
    setQuitConfirmation: () => console.log('[Browser] Quit confirmation setting (no-op)'),
    getDockIconState: (callback) => callback && callback(false),
    getMenuBarIconState: (callback) => callback && callback(false),
    getQuitConfirmationState: (callback) => callback && callback(false),
    openNotificationsSettings: () => console.log('[Browser] Notification settings (no-op)'),
    
    // Updates
    getVersion: () => Promise.resolve('web-version'),
    checkForUpdates: () => console.log('[Browser] Update checking (no-op)'),
    downloadUpdate: () => console.log('[Browser] Update downloading (no-op)'),
    installUpdate: () => console.log('[Browser] Update installation (no-op)'),
    getUpdateState: (callback) => callback && callback({ available: false }),
    onUpdaterEvent: () => {},
    
    // Scheduler
    getSettings: () => Promise.resolve({}),
    setSchedulingEngine: () => console.log('[Browser] Scheduling engine setting (no-op)'),
    
    // Dialog
    showMessageBox: () => {
      console.log('[Browser] Message box not available in browser');
      return Promise.resolve({ response: 0 });
    },
    
    // IPC
    ipcRenderer: {
      send: () => console.warn('IPC not available in web version'),
      invoke: () => Promise.resolve(null),
      on: () => {},
      removeAllListeners: () => {}
    }
  };
</script>`;

        html = html.replace('</head>', `${configScript}</head>`);

        res.send(html);
      } catch (error) {
        log.error('Error serving index.html:', error);
        res.status(500).send('Error loading application');
      }
    });
  }

  webServer = app.listen(port, '127.0.0.1', () => {
    log.info(`Goose web server started on http://127.0.0.1:${port}`);
    console.log(`🌐 Goose is now available at: http://127.0.0.1:${port}`);
  });

  webServer.on('error', (error) => {
    log.error('Web server error:', error);
  });

  return port;
};

export const stopWebServer = (): Promise<void> => {
  return new Promise((resolve) => {
    if (webServer) {
      webServer.close(() => {
        log.info('Web server stopped');
        webServer = null;
        resolve();
      });
    } else {
      resolve();
    }
  });
};

export const isWebServerRunning = (): boolean => {
  return webServer !== null;
};
