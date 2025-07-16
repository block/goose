import React, { useState } from 'react';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '../ui/dialog';
import { Button } from '../ui/button';
import { Session } from '../../sessions';
import { Project } from '../../projects';
import { addSessionToProject } from '../../projects';
import { toastError, toastSuccess } from '../../toasts';
import { ScrollArea } from '../ui/scroll-area';
import { Checkbox } from '../ui/checkbox';
import { formatDistanceToNow } from 'date-fns';

interface AddSessionToProjectModalProps {
  isOpen: boolean;
  onClose: () => void;
  project: Project;
  availableSessions: Session[];
  onSessionsAdded: () => void;
}

const AddSessionToProjectModal: React.FC<AddSessionToProjectModalProps> = ({
  isOpen,
  onClose,
  project,
  availableSessions,
  onSessionsAdded,
}) => {
  const [selectedSessions, setSelectedSessions] = useState<string[]>([]);
  const [isSubmitting, setIsSubmitting] = useState(false);

  const handleToggleSession = (sessionId: string) => {
    setSelectedSessions((prev) => {
      if (prev.includes(sessionId)) {
        return prev.filter((id) => id !== sessionId);
      } else {
        return [...prev, sessionId];
      }
    });
  };

  const handleClose = () => {
    setSelectedSessions([]);
    onClose();
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (selectedSessions.length === 0) {
      toastError({ title: 'Error', msg: 'Please select at least one session' });
      return;
    }

    setIsSubmitting(true);

    try {
      // Add each selected session to the project
      const promises = selectedSessions.map((sessionId) =>
        addSessionToProject(project.id, sessionId)
      );

      await Promise.all(promises);

      toastSuccess({
        title: 'Success',
        msg: `Added ${selectedSessions.length} ${selectedSessions.length === 1 ? 'session' : 'sessions'} to project`,
      });
      onSessionsAdded();
      handleClose();
    } catch (err) {
      console.error('Failed to add sessions to project:', err);
      toastError({ title: 'Error', msg: 'Failed to add sessions to project' });
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <Dialog open={isOpen} onOpenChange={handleClose}>
      <DialogContent className="sm:max-w-[500px]">
        <form onSubmit={handleSubmit}>
          <DialogHeader>
            <DialogTitle>Add Sessions to Project</DialogTitle>
            <DialogDescription>Select sessions to add to "{project.name}"</DialogDescription>
          </DialogHeader>

          {availableSessions.length === 0 ? (
            <div className="py-6 text-center">
              <p className="text-muted-foreground">
                No available sessions to add. All sessions are already part of this project.
              </p>
            </div>
          ) : (
            <ScrollArea className="h-[300px] mt-4 pr-4">
              <div className="space-y-2">
                {availableSessions.map((session) => (
                  <div
                    key={session.id}
                    className="flex items-center space-x-3 py-2 px-3 rounded-md hover:bg-muted/50"
                  >
                    <Checkbox
                      id={`session-${session.id}`}
                      checked={selectedSessions.includes(session.id)}
                      onCheckedChange={() => handleToggleSession(session.id)}
                    />
                    <div className="flex-1 overflow-hidden">
                      <label
                        htmlFor={`session-${session.id}`}
                        className="text-sm font-medium leading-none cursor-pointer flex justify-between w-full"
                      >
                        <span className="truncate">{session.metadata.description}</span>
                        <span className="text-xs text-muted-foreground whitespace-nowrap">
                          {formatDistanceToNow(new Date(session.modified))} ago
                        </span>
                      </label>
                      <p className="text-xs text-muted-foreground truncate mt-1">
                        {session.metadata.working_dir}
                      </p>
                    </div>
                  </div>
                ))}
              </div>
            </ScrollArea>
          )}

          <DialogFooter className="mt-6">
            <Button variant="outline" onClick={handleClose} type="button">
              Cancel
            </Button>
            <Button type="submit" disabled={isSubmitting || selectedSessions.length === 0}>
              {isSubmitting ? 'Adding...' : 'Add Sessions'}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
};

export default AddSessionToProjectModal;
