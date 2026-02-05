import { Recipe } from '../../../recipe';

export type RecipeBuilderView = 'chat' | 'edit';

export interface RecipeBuilderState {
  currentView: RecipeBuilderView;
  testPanelOpen: boolean;
  recipe: Recipe | null;
  testSessionId: string | null;
  testRecipeSnapshot: Recipe | null;
}

export interface RecipeBuilderHeaderProps {
  currentView: RecipeBuilderView;
  onViewChange: (view: RecipeBuilderView) => void;
  testPanelOpen: boolean;
  onTestPanelToggle: () => void;
  canTest: boolean;
  canSave: boolean;
  isSaving: boolean;
  onSave: () => void;
  onClose: () => void;
}

export interface RecipeBuilderChatProps {
  recipe: Recipe | null;
  onRecipeChange: (recipe: Recipe) => void;
}

export interface RecipeBuilderTestProps {
  recipe: Recipe;
  testRecipeSnapshot: Recipe | null;
  onStart: () => void;
  onRestart: () => void;
  isOpen: boolean;
}
