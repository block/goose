import { useRef, useEffect } from 'react';
import { Geese } from '../icons/Geese';
import { Button } from '../ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '../ui/dialog';

interface SessionSummaryModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSave: (editedContent: string) => void;
  summaryContent: string;
}

export function SessionSummaryModal({
  isOpen,
  onClose,
  onSave,
  summaryContent,
}: SessionSummaryModalProps) {
  // Use a ref for the textarea for uncontrolled component
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  // Initialize the textarea value when the modal opens
  useEffect(() => {
    if (isOpen && textareaRef.current) {
      textareaRef.current.value = summaryContent;
    }
  }, [isOpen, summaryContent]);

  // Handle Save action with the edited content from the ref
  const handleSave = () => {
    const currentText = textareaRef.current ? textareaRef.current.value : '';
    onSave(currentText);
  };

  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="sm:max-w-[640px] max-h-[85vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle className="flex flex-col items-center text-center">
            <div className="mb-4">
              <Geese width="48" height="50" />
            </div>
            Session Summary
          </DialogTitle>
          <DialogDescription className="text-center max-w-md m-auto">
            This summary was created to manage your context limit. Review and edit to keep your
            session running smoothly with the information that matters most.
          </DialogDescription>
        </DialogHeader>

        <div className="py-4">
          <div className="w-full">
            <h3 className="text-base font-medium text-gray-900 dark:text-white mb-3">
              Summarization
            </h3>

            <textarea
              ref={textareaRef}
              defaultValue={summaryContent}
              className="bg-gray-50 dark:bg-gray-800 p-4 rounded-lg text-gray-700 dark:text-gray-300 border border-gray-200 dark:border-gray-700 text-sm w-full min-h-[200px] focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
              style={{
                textRendering: 'optimizeLegibility',
                WebkitFontSmoothing: 'antialiased',
                MozOsxFontSmoothing: 'grayscale',
                transform: 'translateZ(0)', // Force hardware acceleration
              }}
            />
          </div>
        </div>

        <DialogFooter className="pt-2">
          <Button variant="outline" onClick={onClose}>
            Cancel
          </Button>
          <Button onClick={handleSave}>Save and Continue</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
