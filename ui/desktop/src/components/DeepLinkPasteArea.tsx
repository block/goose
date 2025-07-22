import { useState, useRef, useEffect, useCallback } from 'react';
import { toast } from 'react-toastify';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from './ui/dialog';
import { Button } from './ui/button';

interface DeepLinkPasteAreaProps {
  isOpen: boolean;
  onClose: () => void;
  onDeepLinkSubmit: (deepLink: string) => void;
}

export const DeepLinkPasteArea: React.FC<DeepLinkPasteAreaProps> = ({
  isOpen,
  onClose,
  onDeepLinkSubmit,
}) => {
  const [inputValue, setInputValue] = useState('');
  const [isProcessing, setIsProcessing] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  // Add debugging
  useEffect(() => {
    console.log('DeepLinkPasteArea props changed:', JSON.stringify({ isOpen }));
  }, [isOpen]);

  useEffect(() => {
    console.log('DeepLinkPasteArea component mounted/updated');
  });

  // Focus input when modal opens
  useEffect(() => {
    if (isOpen && inputRef.current) {
      console.log('DeepLinkPasteArea opening, focusing input');
      // Small delay to ensure modal is fully rendered
      setTimeout(() => {
        inputRef.current?.focus();
      }, 100);
      // Clear any existing value when opening
      setInputValue('');
    }
  }, [isOpen]);

  const handleSubmit = useCallback(async () => {
    const trimmedValue = inputValue.trim();

    if (!trimmedValue) {
      toast.error('Please enter a deeplink');
      return;
    }

    // Validate that it's a goose:// deeplink
    if (!trimmedValue.startsWith('goose://')) {
      toast.error('Invalid deeplink format. Must start with goose://');
      return;
    }

    try {
      setIsProcessing(true);
      await onDeepLinkSubmit(trimmedValue);
      onClose();
    } catch (error) {
      console.error('Error processing deeplink:', error);
      toast.error(
        `Failed to process deeplink: ${error instanceof Error ? error.message : 'Unknown error'}`
      );
    } finally {
      setIsProcessing(false);
    }
  }, [inputValue, onDeepLinkSubmit, onClose]);

  // Handle keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (!isOpen) return;

      if (event.key === 'Escape') {
        event.preventDefault();
        onClose();
      } else if (event.key === 'Enter' && !event.shiftKey) {
        event.preventDefault();
        handleSubmit();
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, inputValue, handleSubmit, onClose]);

  const handlePaste = async () => {
    try {
      const text = await navigator.clipboard.readText();
      setInputValue(text);
      if (text.startsWith('goose://')) {
        // Auto-submit if it's a valid deeplink
        setTimeout(() => handleSubmit(), 100);
      }
    } catch (error) {
      console.error('Failed to read clipboard:', error);
      toast.error('Failed to read from clipboard');
    }
  };

  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="sm:max-w-[500px]">
        <DialogHeader>
          <DialogTitle>Paste Deeplink</DialogTitle>
          <DialogDescription>
            Enter or paste a Goose deeplink to open shared sessions, install extensions, or access
            other features.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          <div>
            <label
              htmlFor="deeplink-input"
              className="block text-sm font-medium text-text-secondary mb-2"
            >
              Deeplink URL:
            </label>
            <input
              ref={inputRef}
              id="deeplink-input"
              type="text"
              value={inputValue}
              onChange={(e) => setInputValue(e.target.value)}
              placeholder="goose://..."
              className="w-full px-3 py-2 border border-border-default rounded-md bg-background-input text-text-primary placeholder-text-secondary focus:outline-none focus:ring-2 focus:ring-accent-primary focus:border-transparent"
              disabled={isProcessing}
            />
          </div>

          <div className="flex justify-between items-center">
            <Button variant="outline" onClick={handlePaste} disabled={isProcessing} size="sm">
              Paste from Clipboard
            </Button>

            <div className="text-xs text-text-secondary">
              <p>
                <strong>Shortcuts:</strong>{' '}
                <kbd className="px-1 py-0.5 bg-background-secondary rounded text-xs">Enter</kbd> to
                submit,{' '}
                <kbd className="px-1 py-0.5 bg-background-secondary rounded text-xs">Esc</kbd> to
                close
              </p>
            </div>
          </div>
        </div>

        <DialogFooter className="pt-2">
          <Button variant="outline" onClick={onClose} disabled={isProcessing}>
            Cancel
          </Button>
          <Button onClick={handleSubmit} disabled={isProcessing || !inputValue.trim()}>
            {isProcessing ? 'Processing...' : 'Submit'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};
