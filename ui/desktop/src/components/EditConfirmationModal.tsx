import React from 'react';
import { AlertTriangle } from 'lucide-react';
import { Button } from './ui/button';

interface EditConfirmationModalProps {
  isOpen: boolean;
  onClose: () => void;
  onContinue: () => void;
  onContinueNewSession: () => void;
}

export const EditConfirmationModal: React.FC<EditConfirmationModalProps> = ({
  isOpen,
  onClose,
  onContinue,
  onContinueNewSession,
}) => {
  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-[400] flex items-center justify-center bg-black/80">
      <div className="bg-background-default border border-borderSubtle rounded-lg w-[90vw] max-w-md">
        {/* Header */}
        <div className="flex items-center gap-3 p-6 border-b border-borderSubtle">
          <AlertTriangle className="w-5 h-5 text-yellow-500" />
          <h2 className="text-lg font-medium text-textProminent">Edit Message</h2>
        </div>

        {/* Content */}
        <div className="p-6">
          <p className="text-textStandard mb-4">
            Editing this message will reset the conversation to the state before this message was
            sent.
          </p>
          <p className="text-textSubtle text-sm">
            All messages after this point will be removed. You can choose to continue in the current
            session or start a new session to preserve the current conversation history.
          </p>
        </div>

        {/* Footer */}
        <div className="flex justify-end gap-3 p-6 border-t border-borderSubtle">
          <Button onClick={onClose} variant="ghost" className="px-4 py-2">
            Cancel
          </Button>
          <Button onClick={onContinueNewSession} variant="outline" className="px-4 py-2">
            Continue in New Session
          </Button>
          <Button
            onClick={onContinue}
            variant="default"
            className="px-4 py-2 bg-background-accent text-text-on-accent hover:opacity-90"
          >
            Continue
          </Button>
        </div>
      </div>
    </div>
  );
};

export default EditConfirmationModal;
