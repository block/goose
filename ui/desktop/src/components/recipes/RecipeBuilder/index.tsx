import { useState, useCallback } from 'react';
import { Recipe } from '../../../recipe';
import { MainPanelLayout } from '../../Layout/MainPanelLayout';
import { saveRecipe } from '../../../recipe/recipe_management';
import { toastSuccess, toastError } from '../../../toasts';
import { errorMessage } from '../../../utils/conversionUtils';
import RecipeBuilderHeader from './RecipeBuilderHeader';
import RecipeBuilderEdit from './RecipeBuilderEdit';
import RecipeBuilderChat from './RecipeBuilderChat';
import RecipeBuilderTest from './RecipeBuilderTest';
import { RecipeBuilderView } from './types';

interface RecipeBuilderProps {
  onClose: () => void;
  onRecipeSaved?: (recipeId: string) => void;
}

export default function RecipeBuilder({ onClose, onRecipeSaved }: RecipeBuilderProps) {
  const [currentView, setCurrentView] = useState<RecipeBuilderView>('chat');
  const [testPanelOpen, setTestPanelOpen] = useState(false);
  const [recipe, setRecipe] = useState<Recipe | null>(null);
  const [isSaving, setIsSaving] = useState(false);
  // Track whether the user has edited the recipe in the Edit view (not yet synced to Chat)
  const [recipeEditedInEditView, setRecipeEditedInEditView] = useState(false);
  // Track the recipe version when test was started (for change detection)
  const [testRecipeSnapshot, setTestRecipeSnapshot] = useState<Recipe | null>(null);

  const canTest = recipe !== null;
  const canSave = recipe !== null && !!recipe.title && !!recipe.description;

  const handleSave = useCallback(async () => {
    if (!recipe) return;

    try {
      setIsSaving(true);
      const recipeId = await saveRecipe(recipe);
      toastSuccess({
        title: 'Recipe saved',
        msg: `"${recipe.title}" has been saved successfully`,
      });
      onRecipeSaved?.(recipeId);
    } catch (error) {
      toastError({
        title: 'Save failed',
        msg: errorMessage(error, 'Failed to save recipe'),
      });
    } finally {
      setIsSaving(false);
    }
  }, [recipe, onRecipeSaved]);

  const handleTestPanelToggle = useCallback(() => {
    setTestPanelOpen((prev) => {
      // Clear snapshot when closing panel
      if (prev) {
        setTestRecipeSnapshot(null);
      }
      return !prev;
    });
  }, []);

  const handleTestStart = useCallback(() => {
    // Take a snapshot of the current recipe when test starts
    setTestRecipeSnapshot(recipe);
  }, [recipe]);

  const handleTestRestart = useCallback(() => {
    // Update snapshot to current recipe on restart
    setTestRecipeSnapshot(recipe);
  }, [recipe]);

  return (
    <MainPanelLayout backgroundColor="bg-background-muted" removeTopPadding={true}>
      <div className="flex flex-col h-full">
        <RecipeBuilderHeader
          currentView={currentView}
          onViewChange={setCurrentView}
          testPanelOpen={testPanelOpen}
          onTestPanelToggle={handleTestPanelToggle}
          canTest={canTest}
          canSave={canSave}
          isSaving={isSaving}
          onSave={handleSave}
          onClose={onClose}
        />

        <div className="flex flex-1 min-h-0">
          <div
            className={`relative flex flex-col flex-1 min-h-0 transition-all duration-300 ${
              testPanelOpen ? 'w-1/2 border-r border-borderSubtle' : 'w-full'
            }`}
          >
            <div
              className={`absolute inset-0 flex flex-col ${currentView !== 'chat' ? 'hidden' : ''}`}
            >
              <RecipeBuilderChat
                recipe={recipe}
                onRecipeChange={setRecipe}
                recipeEditedInEditView={recipeEditedInEditView}
                onRecipeEditSynced={() => setRecipeEditedInEditView(false)}
              />
            </div>
            <div
              className={`absolute inset-0 flex flex-col ${currentView !== 'edit' ? 'hidden' : ''}`}
            >
              <RecipeBuilderEdit
                recipe={recipe}
                onRecipeChange={(r) => {
                  setRecipe(r);
                  setRecipeEditedInEditView(true);
                }}
              />
            </div>
          </div>

          {testPanelOpen && recipe && (
            <div className="w-1/2 flex flex-col min-h-0 bg-background-default">
              <RecipeBuilderTest
                recipe={recipe}
                testRecipeSnapshot={testRecipeSnapshot}
                onStart={handleTestStart}
                onRestart={handleTestRestart}
                isOpen={testPanelOpen}
              />
            </div>
          )}
        </div>
      </div>
    </MainPanelLayout>
  );
}
