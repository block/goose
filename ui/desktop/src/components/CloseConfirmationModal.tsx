import { useEffect, useRef, useState } from 'react';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from './ui/dialog';
import { Button } from './ui/button';
import { Switch } from './ui/switch';
import { defineMessages, useIntl } from '../i18n';
import { acpChatSessionStore } from '../acp/chatSessionStore';
import { decideInitialStep, resolveCloseOutcome } from '../utils/closeDecision';
import type { CloseAction } from '../utils/settings';

const i18n = defineMessages({
  title: { id: 'closeDialog.title', defaultMessage: 'Close Goose' },
  prompt: { id: 'closeDialog.prompt', defaultMessage: 'Do you want to:' },
  tray: { id: 'closeDialog.tray', defaultMessage: 'Close to system tray' },
  quit: { id: 'closeDialog.quit', defaultMessage: 'Close immediately' },
  cancel: { id: 'closeDialog.cancel', defaultMessage: 'Cancel' },
  remember: { id: 'closeDialog.remember', defaultMessage: 'Remember my selection' },
  busyTitle: { id: 'closeDialog.busy.title', defaultMessage: 'Sessions still running' },
  busyMessage: {
    id: 'closeDialog.busy.message',
    defaultMessage:
      'A session is still running. If you close now, its in-progress response will be lost. Do you want to close now?',
  },
  busyConfirm: { id: 'closeDialog.busy.confirm', defaultMessage: 'Yes, close now' },
  busyWait: { id: 'closeDialog.busy.wait', defaultMessage: "No, I'll wait" },
});

type Step = 'closed' | 'choice' | 'busy';

/**
 * Renderer side of the window-close flow. The main process intercepts the window close and asks
 * via the 'request-window-close' IPC; this component decides what to do — minimize to tray, quit,
 * or (when a session is busy) confirm first — and reports the verdict back with `resolveWindowClose`.
 * Mount once per window, inside the IntlProvider.
 */
export default function CloseConfirmationModal() {
  const intl = useIntl();
  const [step, setStep] = useState<Step>('closed');
  const [remember, setRemember] = useState(false);
  const isBusyRef = useRef(false);
  // Guards against double-resolving: closing the Dialog (button click or Esc) both fire, and
  // a stale Esc must not send a second verdict after a button already settled the decision.
  const settledRef = useRef(false);

  const resolve = (action: 'tray' | 'quit' | 'abort') => {
    if (settledRef.current) {
      return;
    }
    settledRef.current = true;
    setStep('closed');
    setRemember(false);
    window.electron.resolveWindowClose(action);
  };

  const handleChoice = (action: 'tray' | 'quit') => {
    if (remember) {
      void window.electron.setSetting('closeAction', action);
    }
    const outcome = resolveCloseOutcome(action, isBusyRef.current);
    if (outcome === 'confirm-busy') {
      setStep('busy');
      return;
    }
    resolve(outcome === 'hide-to-tray' ? 'tray' : 'quit');
  };

  useEffect(() => {
    const handleRequest = async () => {
      // Tell main a live modal is handling this close, so it cancels its no-answer force-close
      // fallback and waits for the user's decision however long it takes.
      window.electron.ackWindowClose();
      settledRef.current = false;
      setRemember(false);
      isBusyRef.current = acpChatSessionStore.hasAnyBusySession();

      let closeAction: CloseAction = 'ask';
      try {
        closeAction = (await window.electron.getSetting('closeAction')) ?? 'ask';
      } catch {
        closeAction = 'ask';
      }

      switch (decideInitialStep(closeAction, isBusyRef.current)) {
        case 'show-choice':
          setStep('choice');
          break;
        case 'confirm-busy':
          setStep('busy');
          break;
        case 'hide-to-tray':
          resolve('tray');
          break;
        case 'quit':
          resolve('quit');
          break;
      }
    };

    window.electron.on('request-window-close', handleRequest);
    return () => window.electron.off('request-window-close', handleRequest);
  }, []);

  return (
    <Dialog open={step !== 'closed'} onOpenChange={(open) => !open && resolve('abort')}>
      <DialogContent className="sm:max-w-[440px]">
        {step === 'busy' ? (
          <>
            <DialogHeader>
              <DialogTitle>{intl.formatMessage(i18n.busyTitle)}</DialogTitle>
              <DialogDescription>{intl.formatMessage(i18n.busyMessage)}</DialogDescription>
            </DialogHeader>
            <DialogFooter className="pt-2">
              <Button variant="outline" onClick={() => resolve('abort')}>
                {intl.formatMessage(i18n.busyWait)}
              </Button>
              <Button variant="destructive" onClick={() => resolve('quit')}>
                {intl.formatMessage(i18n.busyConfirm)}
              </Button>
            </DialogFooter>
          </>
        ) : (
          <>
            <DialogHeader>
              <DialogTitle>{intl.formatMessage(i18n.title)}</DialogTitle>
              <DialogDescription>{intl.formatMessage(i18n.prompt)}</DialogDescription>
            </DialogHeader>
            <label className="flex items-center gap-2 text-sm text-text-secondary cursor-pointer">
              <Switch checked={remember} onCheckedChange={setRemember} variant="mono" />
              <span>{intl.formatMessage(i18n.remember)}</span>
            </label>
            <DialogFooter className="pt-2">
              <Button variant="ghost" onClick={() => resolve('abort')}>
                {intl.formatMessage(i18n.cancel)}
              </Button>
              <Button variant="outline" onClick={() => handleChoice('quit')}>
                {intl.formatMessage(i18n.quit)}
              </Button>
              <Button variant="default" onClick={() => handleChoice('tray')}>
                {intl.formatMessage(i18n.tray)}
              </Button>
            </DialogFooter>
          </>
        )}
      </DialogContent>
    </Dialog>
  );
}
