import { Session, startAgent } from './api';
import { setViewType } from './hooks/useNavigation';

export function resumeSession(session: Session, setView: setViewType) {
  if (process.env.ALPHA) {
    setView('pair', {
      disableAnimation: true,
      resumeSessionId: session.id,
    });
  } else {
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
}

export async function startNewSession(
  initialText: string | undefined,
  resetChat: (() => void) | null,
  setView: setViewType
) {
  if (!resetChat || process.env.ALPHA) {
    const newAgent = await startAgent({
      body: {
        working_dir: window.appConfig.get('GOOSE_WORKING_DIR') as string,
      },
      throwOnError: true,
    });
    const session = newAgent.data;
    setView('pair', {
      disableAnimation: true,
      initialMessage: initialText,
      resumeSessionId: session.id,
    });
  } else {
    resetChat();
    setView('pair', {
      disableAnimation: true,
      initialMessage: initialText,
    });
  }
}
