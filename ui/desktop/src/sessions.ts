import { Session } from './api';
import { getSessionName } from './utils/sessionCompat';

export function resumeSession(session: Session) {
  console.log('Launching session in new window:', getSessionName(session));
  const workingDir = session.working_dir;
  if (!workingDir) {
    throw new Error('Cannot resume session: working directory is missing in session');
  }

  window.electron.createChatWindow(
    undefined, // query
    workingDir,
    undefined, // version
    session.id
  );
}
