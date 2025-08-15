import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from './dialog';
import { Button } from './button';
import MarkdownContent from '../MarkdownContent';

interface RecipeWarningModalProps {
  isOpen: boolean;
  onConfirm: () => void;
  onCancel: () => void;
  recipeDetails: {
    title?: string;
    description?: string;
    instructions?: string;
  };
  hasSecurityWarnings?: boolean;
}

export function RecipeWarningModal({
  isOpen,
  onConfirm,
  onCancel,
  recipeDetails,
  hasSecurityWarnings = false,
}: RecipeWarningModalProps) {
  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && onCancel()}>
      <DialogContent className="sm:max-w-[80vw] max-h-[80vh] flex flex-col p-0">
        <DialogHeader className="flex-shrink-0 p-6 pb-0">
          <DialogTitle>
            {hasSecurityWarnings ? '⚠️ Security Warning' : '⚠️ New Recipe Warning'}
          </DialogTitle>
          <DialogDescription>
            You are about to execute a recipe that you haven't run before. Only proceed if you trust
            the source of this recipe.
          </DialogDescription>
        </DialogHeader>

        {hasSecurityWarnings && (
          <div className="px-6">
            <div className="bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-800 rounded-lg p-4">
              <div className="flex items-start">
                <div className="flex-shrink-0">
                  <svg className="h-5 w-5 text-yellow-400" viewBox="0 0 20 20" fill="currentColor">
                    <path
                      fillRule="evenodd"
                      d="M8.257 3.099c.765-1.36 2.722-1.36 3.486 0l5.58 9.92c.75 1.334-.213 2.98-1.742 2.98H4.42c-1.53 0-2.493-1.646-1.743-2.98l5.58-9.92zM11 13a1 1 0 11-2 0 1 1 0 012 0zm-1-8a1 1 0 00-1 1v3a1 1 0 002 0V6a1 1 0 00-1-1z"
                      clipRule="evenodd"
                    />
                  </svg>
                </div>
                <div className="ml-3">
                  <h3 className="text-sm font-medium text-yellow-800 dark:text-yellow-200">
                    Suspicious Content Detected
                  </h3>
                  <div className="mt-2 text-sm text-yellow-700 dark:text-yellow-300">
                    <p>
                      This recipe contained invisible Unicode characters that could be used for
                      malicious purposes. These characters will be automatically ignored, but you
                      should verify the recipe source is trustworthy.
                    </p>
                  </div>
                </div>
              </div>
            </div>
          </div>
        )}

        <div className="flex-1 overflow-y-auto p-6 pt-4">
          <div className="bg-background-muted p-4 rounded-lg">
            <h3 className="font-medium mb-3 text-text-standard">Recipe Preview:</h3>
            <div className="space-y-4">
              {recipeDetails.title && (
                <p className="text-text-standard">
                  <strong>Title:</strong> {recipeDetails.title}
                </p>
              )}
              {recipeDetails.description && (
                <p className="text-text-standard">
                  <strong>Description:</strong> {recipeDetails.description}
                </p>
              )}
              {recipeDetails.instructions && (
                <div>
                  <h4 className="font-medium text-text-standard mb-1">Instructions:</h4>
                  <MarkdownContent content={recipeDetails.instructions} className="text-sm" />
                </div>
              )}
            </div>
          </div>
        </div>

        <DialogFooter className="flex-shrink-0 p-6 pt-0">
          <Button variant="outline" onClick={onCancel}>
            Cancel
          </Button>
          <Button onClick={onConfirm}>
            {hasSecurityWarnings ? 'Trust and Continue' : 'Trust and Execute'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
