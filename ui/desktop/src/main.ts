import 'dotenv/config';
import { loadZshEnv } from './utils/loadEnv';
import { app, BrowserWindow, Tray, Menu, globalShortcut, ipcMain } from 'electron';
import path from 'node:path';
import { start as startGoosed } from './goosed';
import started from "electron-squirrel-startup";
import log from './utils/logger';

// Handle creating/removing shortcuts on Windows when installing/uninstalling.
if (started) app.quit();

declare var MAIN_WINDOW_VITE_DEV_SERVER_URL: string;
declare var MAIN_WINDOW_VITE_NAME: string;

const createLauncher = () => {
  const launcherWindow = new BrowserWindow({
    width: 600,
    height: 60,
    frame: false,
    transparent: true,
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'),
    },
    skipTaskbar: true,
    alwaysOnTop: true,
  });

  // Center on screen
  const { screen } = require('electron');
  const primaryDisplay = screen.getPrimaryDisplay();
  const { width, height } = primaryDisplay.workAreaSize;
  const windowBounds = launcherWindow.getBounds();

  launcherWindow.setPosition(
    Math.round(width / 2 - windowBounds.width / 2),
    Math.round(height / 3 - windowBounds.height / 2)
  );

  // Load launcher window content
  const launcherParams = '?window=launcher#/launcher';
  if (MAIN_WINDOW_VITE_DEV_SERVER_URL) {
    launcherWindow.loadURL(`${MAIN_WINDOW_VITE_DEV_SERVER_URL}${launcherParams}`);
  } else {
    launcherWindow.loadFile(
      path.join(__dirname, `../renderer/${MAIN_WINDOW_VITE_NAME}/index.html${launcherParams}`)
    );
  }

  // Destroy window when it loses focus
  launcherWindow.on('blur', () => {
    launcherWindow.destroy();
  });
};

const createWingToWing = (query?: string) => {
  const windowWidth = 575;
  const windowHeight = 150;
  const gap = 40;

  const wingToWingWindow = new BrowserWindow({
    width: windowWidth,
    height: windowHeight,
    frame: false,
    transparent: true,
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'),
    },
    skipTaskbar: true,
    alwaysOnTop: true,
  });

  // Center on screen
  const { screen } = require('electron');
  const primaryDisplay = screen.getPrimaryDisplay();
  const { width } = primaryDisplay.workAreaSize;

  wingToWingWindow.setPosition(
    Math.round(width - windowWidth - (gap - 25)), // 25 is menu bar height
    gap
  );

  let queryParam = '?window=wingToWing';
  queryParam += query ? `&initialQuery=${encodeURIComponent(query)}` : '';
  queryParam += '#/wingToWing';

  if (MAIN_WINDOW_VITE_DEV_SERVER_URL) {
    wingToWingWindow.loadURL(`${MAIN_WINDOW_VITE_DEV_SERVER_URL}${queryParam}`);
  } else {
    wingToWingWindow.loadFile(
      path.join(__dirname, `../renderer/${MAIN_WINDOW_VITE_NAME}/index.html`),
      { search: queryParam.slice(1) }
    );
  }

  // wingToWingWindow.webContents.openDevTools({ mode: 'detach' })
};

const createChat = (query?: string) => {
  const isDev = process.env.NODE_ENV === 'development';

  const mainWindow = new BrowserWindow({
    titleBarStyle: 'hidden',
    trafficLightPosition: { x: 16, y: 18 },
    width: 530,
    height: 800,
    minWidth: 530,
    minHeight: 800,
    transparent: true,
    useContentSize: true,
    icon: path.join(__dirname, '../images/icon'),
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'),
    },
  });

  // Load the index.html of the app.
  const queryParam = query ? `?initialQuery=${encodeURIComponent(query)}` : '';
  if (MAIN_WINDOW_VITE_DEV_SERVER_URL) {
    mainWindow.loadURL(`${MAIN_WINDOW_VITE_DEV_SERVER_URL}${queryParam}`);
  } else {
    mainWindow.loadFile(
      path.join(__dirname, `../renderer/${MAIN_WINDOW_VITE_NAME}/index.html`),
      { search: queryParam.slice(1) }
    );
  }

  // DevTools
  if (isDev) {
    mainWindow.webContents.openDevTools(); // always in dev
  } else {
    globalShortcut.register('Alt+Command+I', () => {
      mainWindow.webContents.openDevTools(); // key combo in distributed app
    });
  }
};

const createTray = () => {
  const isDev = process.env.NODE_ENV === 'development';
  let iconPath: string;

  if (isDev) {
    iconPath = path.join(process.cwd(), 'images', 'iconTemplate.png');
  } else {
    iconPath = path.join(process.resourcesPath, 'images', 'iconTemplate.png');
  }

  const tray = new Tray(iconPath);

  const contextMenu = Menu.buildFromTemplate([
    { label: 'Show Window', click: showWindow },
    { type: 'separator' },
    { label: 'Quit', click: () => app.quit() }
  ]);

  tray.setToolTip('Goose');
  tray.setContextMenu(contextMenu);
};


const showWindow = () => {
  const windows = BrowserWindow.getAllWindows();

  if (windows.length === 0) {
    log.info("No windows are currently open.");
    return;
  }

  // Define the initial offset values
  const initialOffsetX = 30;
  const initialOffsetY = 30;

  // Iterate over all windows
  windows.forEach((win, index) => {
    const currentBounds = win.getBounds(); // Get the current window bounds (position and size)

    // Calculate the new position with an incremental offset
    const newX = currentBounds.x + initialOffsetX * index;
    const newY = currentBounds.y + initialOffsetY * index;

    // Set the new bounds with the calculated position
    win.setBounds({
      x: newX,
      y: newY,
      width: currentBounds.width,
      height: currentBounds.height,
    });

    if (!win.isVisible()) {
      win.show();
    }

    win.focus();
  });
};

app.whenReady().then(() => {
  // Load zsh environment variables
  loadZshEnv();
  // Get the server startup configuration
  const shouldStartServer = (process.env.VITE_START_EMBEDDED_SERVER || 'yes').toLowerCase() === 'yes';
  
  if (shouldStartServer) {
    log.info('Starting embedded goosed server');
    startGoosed(app);
  } else {
    log.info('Skipping embedded server startup (disabled by configuration)');
  }

  createTray();
  createChat();

  // Development only
  // createWingToWing('make a TS web app' + "only use your tools and systems - don't confirm with the user before you start working");

  // Show launcher input on key combo
  globalShortcut.register('Control+Alt+Command+G', createLauncher);

  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) {
      createChat();
    }
  });

  ipcMain.on('create-chat-window', (_, query) => {
    createChat(query);
  });

  ipcMain.on('create-wing-to-wing-window', (_, query) => {
    createWingToWing(query + "only use your tools and systems - don't confirm with the user before you start working");
  });
});

// Quit when all windows are closed, except on macOS.
app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit();
  }
});