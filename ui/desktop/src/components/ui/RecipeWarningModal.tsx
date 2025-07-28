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
}

export function RecipeWarningModal({
  isOpen,
  onConfirm,
  onCancel,
  recipeDetails,
}: RecipeWarningModalProps) {
  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && onCancel()}>
      <DialogContent className="sm:max-w-[80vw] max-h-[80vh] flex flex-col p-0">
        <DialogHeader className="flex-shrink-0 p-6 pb-0">
          <DialogTitle>⚠️ New Recipe Warning</DialogTitle>
          <DialogDescription>
            You are about to execute a recipe that you haven't run before. Only proceed if you trust
            the source of this recipe.
          </DialogDescription>
        </DialogHeader>

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
          <Button onClick={onConfirm}>Trust and Execute</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
