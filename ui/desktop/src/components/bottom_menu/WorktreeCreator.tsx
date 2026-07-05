import React, { useCallback, useEffect, useState } from 'react';
import { GitBranch } from 'lucide-react';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '../ui/Tooltip';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '../ui/dialog';
import { Button } from '../ui/button';
import { Input } from '../ui/input';
import { toast } from 'react-toastify';
import { defineMessages, useIntl } from '../../i18n';

const i18n = defineMessages({
  createWorktree: {
    id: 'worktreeCreator.createWorktree',
    defaultMessage: 'Create a git worktree',
  },
  enterBranchName: {
    id: 'worktreeCreator.enterBranchName',
    defaultMessage: 'Enter a branch name for the new worktree',
  },
  branchName: {
    id: 'worktreeCreator.branchName',
    defaultMessage: 'Branch name',
  },
  branchNamePlaceholder: {
    id: 'worktreeCreator.branchNamePlaceholder',
    defaultMessage: 'my-feature-branch',
  },
  create: {
    id: 'worktreeCreator.create',
    defaultMessage: 'Create',
  },
  cancel: {
    id: 'worktreeCreator.cancel',
    defaultMessage: 'Cancel',
  },
  failedToCreate: {
    id: 'worktreeCreator.failedToCreate',
    defaultMessage: 'Failed to create worktree',
  },
  invalidBranchName: {
    id: 'worktreeCreator.invalidBranchName',
    defaultMessage:
      'Branch name can only contain letters, numbers, hyphens, underscores, dots, and slashes',
  },
});

interface WorktreeCreatorProps {
  sessionId: string | undefined;
  workingDir: string;
  onWorkingDirChange?: (newDir: string) => Promise<void> | void;
  onRestartStart?: () => void;
  onRestartEnd?: () => void;
}

const BRANCH_NAME_PATTERN = /^[\w./-]+$/;

export const WorktreeCreator: React.FC<WorktreeCreatorProps> = ({
  sessionId,
  workingDir,
  onWorkingDirChange,
  onRestartStart,
  onRestartEnd,
}) => {
  const intl = useIntl();
  const [isGitRepo, setIsGitRepoState] = useState(false);
  const [isDialogOpen, setIsDialogOpen] = useState(false);
  const [branchName, setBranchName] = useState('');
  const [isCreating, setIsCreating] = useState(false);

  useEffect(() => {
    let cancelled = false;
    if (!workingDir?.trim()) {
      setIsGitRepoState(false);
      return;
    }
    window.electron
      .isGitRepo(workingDir)
      .then((result) => {
        if (!cancelled) setIsGitRepoState(result);
      })
      .catch(() => {
        if (!cancelled) setIsGitRepoState(false);
      });
    return () => {
      cancelled = true;
    };
  }, [workingDir]);

  const applyDirectoryChange = useCallback(
    async (newDir: string) => {
      window.electron.addRecentDir(newDir);

      if (sessionId) {
        onRestartStart?.();
        try {
          await onWorkingDirChange?.(newDir);
        } catch (error) {
          console.error('[WorktreeCreator] Failed to switch to worktree:', error);
          toast.error(intl.formatMessage(i18n.failedToCreate));
        } finally {
          onRestartEnd?.();
        }
      } else {
        await onWorkingDirChange?.(newDir);
      }
    },
    [sessionId, onRestartStart, onRestartEnd, onWorkingDirChange, intl]
  );

  const handleCreate = async () => {
    const trimmed = branchName.trim();
    if (!trimmed || !BRANCH_NAME_PATTERN.test(trimmed)) {
      toast.error(intl.formatMessage(i18n.invalidBranchName));
      return;
    }

    setIsCreating(true);
    try {
      const worktreePath = await window.electron.createGitWorktree(workingDir, trimmed);
      setIsDialogOpen(false);
      setBranchName('');
      await applyDirectoryChange(worktreePath);
    } catch (error) {
      const message =
        error instanceof Error ? error.message : intl.formatMessage(i18n.failedToCreate);
      toast.error(message);
    } finally {
      setIsCreating(false);
    }
  };

  if (!isGitRepo) {
    return null;
  }

  return (
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger asChild>
          <button
            className="text-text-primary/70 hover:cursor-pointer hover:text-text-primary text-xs flex items-center transition-colors pl-1 [&>svg]:size-4"
            onClick={() => setIsDialogOpen(true)}
          >
            <GitBranch size={16} className="mr-px" />
            <span className="text-[10px] leading-none -ml-px">+</span>
          </button>
        </TooltipTrigger>
        <TooltipContent side="top">{intl.formatMessage(i18n.createWorktree)}</TooltipContent>
      </Tooltip>

      <Dialog open={isDialogOpen} onOpenChange={setIsDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{intl.formatMessage(i18n.createWorktree)}</DialogTitle>
            <DialogDescription>{intl.formatMessage(i18n.enterBranchName)}</DialogDescription>
          </DialogHeader>
          <div className="py-4">
            <label className="text-sm font-medium mb-2 block">
              {intl.formatMessage(i18n.branchName)}
            </label>
            <Input
              value={branchName}
              onChange={(e) => setBranchName(e.target.value)}
              placeholder={intl.formatMessage(i18n.branchNamePlaceholder)}
              onKeyDown={(e) => {
                if (e.key === 'Enter' && !isCreating) {
                  void handleCreate();
                }
              }}
              autoFocus
            />
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setIsDialogOpen(false)} disabled={isCreating}>
              {intl.formatMessage(i18n.cancel)}
            </Button>
            <Button onClick={() => void handleCreate()} disabled={isCreating || !branchName.trim()}>
              {intl.formatMessage(i18n.create)}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </TooltipProvider>
  );
};
