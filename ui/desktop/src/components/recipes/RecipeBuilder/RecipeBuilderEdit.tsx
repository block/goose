import { useEffect, useCallback, useRef } from 'react';
import { useForm } from '@tanstack/react-form';
import { Recipe } from '../../../recipe';
import { ScrollArea } from '../../ui/scroll-area';
import { RecipeFormFields } from '../shared/RecipeFormFields';
import { RecipeFormData } from '../shared/recipeFormSchema';

interface RecipeBuilderEditProps {
  recipe: Recipe | null;
  onRecipeChange: (recipe: Recipe) => void;
}

const emptyFormData: RecipeFormData = {
  title: '',
  description: '',
  instructions: '',
  prompt: '',
  activities: [],
  parameters: [],
  jsonSchema: '',
};

function recipeToFormData(recipe: Recipe | null): RecipeFormData {
  if (!recipe) return emptyFormData;
  return {
    title: recipe.title || '',
    description: recipe.description || '',
    instructions: recipe.instructions || '',
    prompt: recipe.prompt || '',
    activities: recipe.activities || [],
    parameters: recipe.parameters || [],
    jsonSchema: recipe.response?.json_schema
      ? JSON.stringify(recipe.response.json_schema, null, 2)
      : '',
  };
}

function formDataToRecipe(data: RecipeFormData): Recipe {
  let responseConfig = undefined;
  if (data.jsonSchema && data.jsonSchema.trim()) {
    try {
      const parsedSchema = JSON.parse(data.jsonSchema);
      responseConfig = { json_schema: parsedSchema };
    } catch {
      // Invalid JSON, skip
    }
  }

  return {
    version: '1.0.0',
    title: data.title,
    description: data.description,
    instructions: data.instructions || undefined,
    prompt: data.prompt || undefined,
    activities: data.activities.length > 0 ? data.activities : undefined,
    parameters: data.parameters.length > 0 ? data.parameters : undefined,
    response: responseConfig,
  };
}

export default function RecipeBuilderEdit({ recipe, onRecipeChange }: RecipeBuilderEditProps) {
  const lastExternalRecipeRef = useRef<Recipe | null>(null);
  const isInternalUpdateRef = useRef(false);

  const form = useForm({
    defaultValues: recipeToFormData(recipe),
  });

  useEffect(() => {
    const isSameRecipe = JSON.stringify(recipe) === JSON.stringify(lastExternalRecipeRef.current);
    if (!isSameRecipe && !isInternalUpdateRef.current) {
      lastExternalRecipeRef.current = recipe;
      form.reset(recipeToFormData(recipe));
    }
    isInternalUpdateRef.current = false;
  }, [recipe, form]);

  useEffect(() => {
    return form.store.subscribe(() => {
      const formData = form.state.values;
      const newRecipe = formDataToRecipe(formData);

      if (newRecipe.title || newRecipe.description || newRecipe.instructions) {
        isInternalUpdateRef.current = true;
        lastExternalRecipeRef.current = newRecipe;
        onRecipeChange(newRecipe);
      }
    });
  }, [form, onRecipeChange]);

  const handleFieldChange = useCallback(
    (field: keyof RecipeFormData, value: string) => {
      form.setFieldValue(field, value);
    },
    [form]
  );

  return (
    <ScrollArea className="flex-1 h-full">
      <div className="p-6 bg-background-default min-h-full">
        <RecipeFormFields
          form={form}
          onTitleChange={(v) => handleFieldChange('title', v)}
          onDescriptionChange={(v) => handleFieldChange('description', v)}
          onInstructionsChange={(v) => handleFieldChange('instructions', v)}
          onPromptChange={(v) => handleFieldChange('prompt', v)}
          onJsonSchemaChange={(v) => handleFieldChange('jsonSchema', v)}
        />
      </div>
    </ScrollArea>
  );
}
