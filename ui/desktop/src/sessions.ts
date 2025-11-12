import { Session, startAgent } from './api';
import type { setViewType } from './hooks/useNavigation';

export function resumeSession(session: Session, setView: setViewType) {
  setView('pair', {
    disableAnimation: true,
    resumeSessionId: session.id,
  });
}

export async function startNewSession(
  initialText: string | undefined,
  _resetChat: (() => void) | null,
  setView: setViewType
) {
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
}
