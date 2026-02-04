import { useState, useEffect } from 'react';
import { Button } from './ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from './ui/dialog';
import { confirmToolAction, Permission } from '../api';

const globalApprovalState = new Map<string, { isClicked: boolean; decision: Permission | null }>();

export interface ToolApprovalData {
  id: string;
  toolName: string;
  prompt?: string;
}

interface ToolApprovalButtonsProps {
  sessionId: string;
  data: ToolApprovalData;
  isClicked: boolean;
  isCancelled: boolean;
}

export default function ToolApprovalButtons({
  sessionId,
  data,
  isClicked: initialIsClicked,
  isCancelled,
}: ToolApprovalButtonsProps) {
  const { id, toolName } = data;

  const [isClicked, setIsClicked] = useState(() => {
    const savedState = globalApprovalState.get(id);
    return savedState?.isClicked ?? initialIsClicked;
  });
  const [decision, setDecision] = useState<Permission | null>(() => {
    const savedState = globalApprovalState.get(id);
    return savedState?.decision ?? null;
  });
  const [showPermissionModal, setShowPermissionModal] = useState(false);
  const [pendingAction, setPendingAction] = useState<'allow' | 'deny' | null>(null);

  useEffect(() => {
    globalApprovalState.set(id, { isClicked, decision });
  }, [id, isClicked, decision]);

  const handleAction = async (action: Permission) => {
    await confirmToolAction({
      body: {
        sessionId,
        id,
        action,
      },
    });
    setIsClicked(true);
    setDecision(action);
  };

  const handleAllowClick = () => {
    setPendingAction('allow');
    setShowPermissionModal(true);
  };

  const handleDenyClick = () => {
    setPendingAction('deny');
    setShowPermissionModal(true);
  };

  const handleModalConfirm = async (permanent: boolean) => {
    if (pendingAction === 'allow') {
      await handleAction(permanent ? 'always_allow' : 'allow_once');
    } else if (pendingAction === 'deny') {
      await handleAction('always_deny');
    }
    setShowPermissionModal(false);
    setPendingAction(null);
  };

  const handleModalCancel = () => {
    setShowPermissionModal(false);
    setPendingAction(null);
  };

  if (isClicked || isCancelled) {
    let statusMessage = '';
    if (isCancelled) {
      statusMessage = 'Tool confirmation is not available';
    } else if (decision === 'allow_once') {
      statusMessage = 'Allowed once';
    } else if (decision === 'always_allow') {
      statusMessage = 'Always allowed';
    } else if (decision === 'always_deny') {
      statusMessage = 'Denied';
    }

    return <div className="px-4 py-3 text-sm text-textSubtle">{statusMessage}</div>;
  }

  return (
    <>
      <div className="px-4 py-3 flex items-center gap-3">
        <Button
          variant="default"
          size="sm"
          onClick={handleAllowClick}
          className="bg-green-600 hover:bg-green-700 text-white"
        >
          Allow
        </Button>
        <Button
          variant="outline"
          size="sm"
          onClick={handleDenyClick}
          className="border-red-500/50 text-red-500 hover:bg-red-500/10"
        >
          Deny
        </Button>
      </div>

      {/* Permission scope modal */}
      <Dialog open={showPermissionModal} onOpenChange={setShowPermissionModal}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{pendingAction === 'allow' ? 'Allow Tool?' : 'Deny Tool?'}</DialogTitle>
            <DialogDescription>
              {pendingAction === 'allow'
                ? `Choose how to handle "${toolName}" calls.`
                : `Deny "${toolName}" for this request?`}
            </DialogDescription>
          </DialogHeader>
          <DialogFooter className="flex gap-2 sm:gap-0">
            <Button variant="ghost" onClick={handleModalCancel}>
              Cancel
            </Button>
            {pendingAction === 'allow' ? (
              <>
                <Button variant="outline" onClick={() => handleModalConfirm(false)}>
                  Allow Once
                </Button>
                <Button
                  variant="default"
                  onClick={() => handleModalConfirm(true)}
                  className="bg-green-600 hover:bg-green-700"
                >
                  Always Allow
                </Button>
              </>
            ) : (
              <Button variant="destructive" onClick={() => handleModalConfirm(true)}>
                Deny
              </Button>
            )}
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}
