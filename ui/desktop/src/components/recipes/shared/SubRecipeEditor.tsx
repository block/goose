import { useState } from 'react';
import { Plus, Edit2, Trash2, FilePlus } from 'lucide-react';
import { Button } from '../../ui/button';
import { SubRecipeFormData } from './recipeFormSchema';
import SubRecipeModal from './SubRecipeModal';
import CreateSubRecipeInline from './CreateSubRecipeInline';

interface SubRecipeEditorProps {
  subRecipes: SubRecipeFormData[];
  onChange: (subRecipes: SubRecipeFormData[]) => void;
}

export default function SubRecipeEditor({ subRecipes, onChange }: SubRecipeEditorProps) {
  const [showModal, setShowModal] = useState(false);
  const [editingSubRecipe, setEditingSubRecipe] = useState<SubRecipeFormData | null>(null);
  const [editingIndex, setEditingIndex] = useState<number | null>(null);
  const [showCreateRecipeModal, setShowCreateRecipeModal] = useState(false);

  const handleAddSubRecipe = () => {
    setEditingSubRecipe(null);
    setEditingIndex(null);
    setShowModal(true);
  };

  const handleCreateNewRecipe = () => {
    setShowCreateRecipeModal(true);
  };

  const handleEditSubRecipe = (subRecipe: SubRecipeFormData, index: number) => {
    setEditingSubRecipe(subRecipe);
    setEditingIndex(index);
    setShowModal(true);
  };

  const handleDeleteSubRecipe = (index: number) => {
    const newSubRecipes = subRecipes.filter((_, i) => i !== index);
    onChange(newSubRecipes);
  };

  const handleSaveSubRecipe = (subRecipe: SubRecipeFormData) => {
    if (editingIndex !== null) {
      const newSubRecipes = [...subRecipes];
      newSubRecipes[editingIndex] = subRecipe;
      onChange(newSubRecipes);
    } else {
      onChange([...subRecipes, subRecipe]);
    }
  };

  const handleSubRecipeSaved = (subRecipe: SubRecipeFormData) => {
    // Directly add the subrecipe with all its configuration
    onChange([...subRecipes, subRecipe]);
  };

  return (
    <div>
      <div className="flex items-center justify-between mb-2">
        <label className="block text-md text-textProminent font-bold">Subrecipes</label>
        <div className="flex gap-2">
          <Button
            type="button"
            onClick={handleCreateNewRecipe}
            variant="outline"
            size="sm"
            className="flex items-center gap-2"
          >
            <FilePlus className="w-4 h-4" />
            Create New Subrecipe
          </Button>
          <Button
            type="button"
            onClick={handleAddSubRecipe}
            variant="outline"
            size="sm"
            className="flex items-center gap-2"
          >
            <Plus className="w-4 h-4" />
            Add Existing
          </Button>
        </div>
      </div>

      <p className="text-textSubtle text-sm mb-4">
        Subrecipes are recipes that can be called as tools during execution. They enable multi-step
        workflows and reusable components.
      </p>

      {subRecipes.length > 0 && (
        <div className="space-y-2">
          {subRecipes.map((subRecipe, index) => (
            <div
              key={index}
              className="border border-border-subtle rounded-lg p-4 bg-background-default hover:bg-background-muted transition-colors"
            >
              <div className="flex items-start justify-between">
                <div className="flex-1">
                  <div className="flex items-center gap-2 mb-1">
                    <h4 className="text-sm font-semibold text-textProminent">{subRecipe.name}</h4>
                    {subRecipe.sequential_when_repeated && (
                      <span className="text-xs px-2 py-0.5 bg-blue-100 text-blue-700 rounded">
                        Sequential
                      </span>
                    )}
                  </div>
                  <p className="text-xs text-text-muted mb-2">{subRecipe.path}</p>
                  {subRecipe.description && (
                    <p className="text-sm text-text-standard mb-2">{subRecipe.description}</p>
                  )}
                  {subRecipe.values && Object.keys(subRecipe.values).length > 0 && (
                    <div className="mt-2">
                      <p className="text-xs text-text-muted mb-1">Pre-configured values:</p>
                      <div className="flex flex-wrap gap-1">
                        {Object.entries(subRecipe.values).map(([key, value]) => (
                          <span
                            key={key}
                            className="text-xs px-2 py-1 bg-background-muted border border-border-subtle rounded"
                          >
                            <span className="font-medium">{key}</span>
                            <span className="text-text-muted">: </span>
                            <span className="text-text-standard">{value}</span>
                          </span>
                        ))}
                      </div>
                    </div>
                  )}
                </div>
                <div className="flex gap-1 ml-4">
                  <Button
                    type="button"
                    onClick={() => handleEditSubRecipe(subRecipe, index)}
                    variant="ghost"
                    size="sm"
                    className="p-2 hover:bg-blue-100 hover:text-blue-600"
                  >
                    <Edit2 className="w-4 h-4" />
                  </Button>
                  <Button
                    type="button"
                    onClick={() => handleDeleteSubRecipe(index)}
                    variant="ghost"
                    size="sm"
                    className="p-2 hover:bg-red-100 hover:text-red-600"
                  >
                    <Trash2 className="w-4 h-4" />
                  </Button>
                </div>
              </div>
            </div>
          ))}
        </div>
      )}

      <SubRecipeModal
        isOpen={showModal}
        onClose={() => {
          setShowModal(false);
        }}
        onSave={handleSaveSubRecipe}
        subRecipe={editingSubRecipe}
      />

      {/* Create Subrecipe Modal */}
      <CreateSubRecipeInline
        isOpen={showCreateRecipeModal}
        onClose={() => {
          setShowCreateRecipeModal(false);
        }}
        onSubRecipeSaved={handleSubRecipeSaved}
      />
    </div>
  );
}
