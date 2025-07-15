import { Card } from './card';

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
  if (!isOpen) return null;

  return (
    <div
      className="fixed inset-0 bg-black/20 dark:bg-white/20 backdrop-blur-sm transition-colors flex items-center justify-center p-4 z-50"
      onClick={(e) => {
        if (e.target === e.currentTarget) onCancel();
      }}
    >
      <Card className="relative w-[700px] max-w-full bg-bgApp rounded-xl my-10 max-h-[90vh] flex flex-col shadow-lg">
        <div className="p-8 overflow-y-auto" style={{ maxHeight: 'calc(90vh - 32px)' }}>
          <h2 className="text-2xl font-bold mb-4 text-textStandard">⚠️ New Recipe Warning</h2>
          <p className="mb-4 text-textStandard">
            You are about to execute a recipe that you haven't run before. Only proceed if you trust
            the source of this recipe.
          </p>

          <div className="mb-6 bg-bgSubtle p-4 rounded-lg">
            <h3 className="font-medium mb-2 text-textStandard">Recipe Details:</h3>
            <div className="space-y-2">
              {recipeDetails.title && (
                <p className="text-textStandard">
                  <strong>Title:</strong> {recipeDetails.title}
                </p>
              )}
              {recipeDetails.description && (
                <p className="text-textStandard">
                  <strong>Description:</strong> {recipeDetails.description}
                </p>
              )}
              {recipeDetails.instructions && (
                <p className="text-textStandard">
                  <strong>Instructions:</strong> {recipeDetails.instructions}
                </p>
              )}
            </div>
          </div>

          <div className="flex justify-end space-x-4">
            <button
              onClick={onCancel}
              className="px-4 py-2 bg-gray-500 text-white rounded-md hover:bg-gray-600"
            >
              Cancel
            </button>
            <button
              onClick={onConfirm}
              className="px-4 py-2 bg-blue-500 text-white rounded-md hover:bg-blue-600"
            >
              Trust and Execute
            </button>
          </div>
        </div>
      </Card>
    </div>
  );
}
