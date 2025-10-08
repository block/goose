import { Session } from './api';
import { USE_NEW_CHAT } from './updates';

export function resumeSession(
  session: Session,
  navigateInSameWindow?: (sessionId: string) => void
) {
  const workingDir = session.working_dir;
  if (!workingDir) {
    throw new Error('Cannot resume session: working directory is missing in session');
  }

  // When USE_NEW_CHAT is true and we have a navigation callback, resume in the same window
  // Otherwise, open in a new window (old behavior)
  if (USE_NEW_CHAT && navigateInSameWindow) {
    navigateInSameWindow(session.id);
  } else {
    window.electron.createChatWindow(
      undefined, // query
      workingDir,
      undefined, // version
      session.id
    );
  }
}
