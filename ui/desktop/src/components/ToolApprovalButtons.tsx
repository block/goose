import { useState, useEffect } from 'react';
import { Button } from './ui/button';
import { confirmToolAction, Permission } from '../api';
import { useTranslation } from 'react-i18next';

const globalApprovalState = new Map<
  string,
  {
    decision: Permission | null;
    isClicked: boolean;
  }
>();

export interface ToolApprovalData {
  id: string;
  toolName: string;
  prompt?: string;
  sessionId: string;
  isClicked?: boolean;
}

export default function ToolApprovalButtons({ data }: { data: ToolApprovalData }) {
  const { t } = useTranslation();
  const { id, toolName, prompt, sessionId, isClicked: initialIsClicked } = data;

  const storedState = globalApprovalState.get(id);
  const [decision, setDecision] = useState<Permission | null>(storedState?.decision ?? null);
  const [isClicked, setIsClicked] = useState(storedState?.isClicked ?? initialIsClicked ?? false);

  useEffect(() => {
    const currentState = globalApprovalState.get(id);
    if (currentState) {
      setDecision(currentState.decision);
      setIsClicked(currentState.isClicked);
    }
  }, [id]);

  useEffect(() => {
    globalApprovalState.set(id, { decision, isClicked });
  }, [id, decision, isClicked]);

  const handleAction = async (action: Permission) => {
    setDecision(action);
    setIsClicked(true);

    try {
      const response = await confirmToolAction({
        body: {
          sessionId,
          id,
          action,
          principalType: 'Tool',
        },
      });
      if (response.error) {
        console.error('Failed to confirm tool action:', response.error);
      }
    } catch (err) {
      console.error('Error confirming tool action:', err);
    }
  };

  if (isClicked && decision) {
    const statusMessages: Record<Permission, string> = {
      allow_once: t('toolApproval.status.allowOnce'),
      always_allow: t('toolApproval.status.alwaysAllow'),
      always_deny: t('toolApproval.status.denied'),
      deny_once: t('toolApproval.status.denyOnce'),
      cancel: t('toolApproval.status.cancelled'),
    };
    return (
      <p className="text-sm text-muted-foreground mt-2">
        {toolName} - {statusMessages[decision]}
      </p>
    );
  }

  return (
    <div className="flex items-center gap-2 mt-2">
      <Button
        className="rounded-full"
        variant="secondary"
        onClick={() => handleAction('allow_once')}
      >
        {t('toolApproval.actions.allowOnce')}
      </Button>
      {!prompt && (
        <Button
          className="rounded-full"
          variant="secondary"
          onClick={() => handleAction('always_allow')}
        >
          {t('toolApproval.actions.alwaysAllow')}
        </Button>
      )}
      <Button className="rounded-full" variant="outline" onClick={() => handleAction('deny_once')}>
        {t('toolApproval.actions.deny')}
      </Button>
    </div>
  );
}
