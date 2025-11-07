import { app, BrowserWindow } from 'electron';
import path from 'node:path';
import { GooseApp } from '../api';
import fs from 'fs';

export function getContainerHtml(gapp: GooseApp): string {
  const jsImplementation = gapp.jsImplementation!;
  const appName = gapp.name;

  const assetsPath = app.isPackaged
    ? path.join(process.resourcesPath, 'src/goose_apps/assets')
    : path.join(__dirname, '../../src/goose_apps/assets');

  console.log('__dirname', __dirname);

  let containerHtml = fs.readFileSync(path.join(assetsPath, 'container.html'), 'utf-8');
  const gooseWidgetJs = fs.readFileSync(path.join(assetsPath, 'goose-widget.js'), 'utf-8');
  const containerJs = fs.readFileSync(path.join(assetsPath, 'container.js'), 'utf-8');

  const asScript = (src: string) => `<script>\n${src}\n</script>`;

  const classMatch = jsImplementation.match(/class\s+(\w+)\s+extends\s+GooseWidget/);
  if (!classMatch) {
    throw new Error('No class extending GooseWidget found in implementation');
  }
  const widgetClassName = classMatch[1];

  const vars: [string, string][] = [
    ['TITLE', appName],
    ['GOOSE_WIDGET_JS', asScript(gooseWidgetJs)],
    ['CONTAINER_JS', asScript(containerJs)],
    [
      'WIDGET_JS',
      asScript(jsImplementation + '\nwindow.' + widgetClassName + ' = ' + widgetClassName + ';'),
    ],
    ['WIDGET_CLASS_NAME', widgetClassName],
  ];

  for (const [key, val] of vars) {
    containerHtml = containerHtml.replace(`{{ ${key} }}`, val);
  }

  return containerHtml;
}

export async function launchGooseApp(gapp: GooseApp): Promise<void> {
  const desiredContentWidth = gapp.width || 800;
  const desiredContentHeight = gapp.height || 600;

  const appWindow = new BrowserWindow({
    title: gapp.name,
    width: desiredContentWidth,
    height: desiredContentHeight,
    resizable: gapp.resizable ?? true,
    webPreferences: {
      nodeIntegration: false,
      contextIsolation: true,
      webSecurity: true,
    },
  });

  const [currentContentWidth, currentContentHeight] = appWindow.getContentSize();
  const [currentWindowWidth, currentWindowHeight] = appWindow.getSize();

  const frameWidth = currentWindowWidth - currentContentWidth;
  const frameHeight = currentWindowHeight - currentContentHeight;

  appWindow.setSize(desiredContentWidth + frameWidth, desiredContentHeight + frameHeight);

  const html = getContainerHtml(gapp);
  await appWindow.loadURL(`data:text/html;charset=utf-8,${encodeURIComponent(html)}`);

  appWindow.show();
}
