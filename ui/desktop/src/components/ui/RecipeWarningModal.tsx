import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from './dialog';
import { Button } from './button';

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
      <DialogContent className="sm:max-w-[600px]">
        <DialogHeader>
          <DialogTitle>⚠️ New Recipe Warning</DialogTitle>
          <DialogDescription>
            You are about to execute a recipe that you haven't run before. Only proceed if you trust
            the source of this recipe.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          <div className="bg-background-muted p-4 rounded-lg">
            <h3 className="font-medium mb-2 text-text-standard">Recipe Details:</h3>
            <div className="space-y-2 text-sm">
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
                <p className="text-text-standard">
                  <strong>Instructions:</strong> {recipeDetails.instructions}
                </p>
              )}
            </div>
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={onCancel}>
            Cancel
          </Button>
          <Button onClick={onConfirm}>Trust and Execute</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
