import { Session } from './api';

export function resumeSession(session: Session) {
  console.log('Launching session in new window:', session.description || session.id);
  const workingDir = session.working_dir;
  if (!workingDir) {
    throw new Error('Cannot resume session: working directory is missing in session metadata');
  }

  window.electron.createChatWindow(
    undefined, // query
    workingDir,
    undefined, // version
    session.id
  );
}
