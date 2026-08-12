import log from 'electron-log';
import path from 'node:path';
import { app } from 'electron';

log.transports.file.resolvePathFn = () => {
  return path.join(app.getPath('userData'), 'logs', 'main.log');
};

log.transports.file.level = app.isPackaged ? 'info' : 'debug';
log.transports.console.level = app.isPackaged ? false : 'debug';

export default log;
