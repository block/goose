import React, { useState } from 'react';
import { Parameter } from '../../../recipe';
import { RecipeNameField } from './RecipeNameField';

import ParameterInput from '../../parameter/ParameterInput';
import RecipeActivityEditor from '../RecipeActivityEditor';
import JsonSchemaEditor from './JsonSchemaEditor';
import InstructionsEditor from './InstructionsEditor';
import { Button } from '../../ui/button';
import { RecipeFormApi } from './recipeFormSchema';
import { extractTemplateVariables } from '../../../utils/providerUtils';

// Type for field API to avoid linting issues - use any to bypass complex type constraints
// eslint-disable-next-line @typescript-eslint/no-explicit-any
type FormFieldApi<_T = any> = any;

interface RecipeFormFieldsProps {
  // Form instance from parent
  form: RecipeFormApi;

  // Event handlers
  onTitleChange?: (value: string) => void;
  onDescriptionChange?: (value: string) => void;
  onInstructionsChange?: (value: string) => void;
  onPromptChange?: (value: string) => void;
  onJsonSchemaChange?: (value: string) => void;
  onRecipeNameChange?: (value: string) => void;
  onGlobalChange?: (value: boolean) => void;
}

export function RecipeFormFields({
  form,
  onTitleChange,
  onDescriptionChange,
  onInstructionsChange,
  onPromptChange,
  onJsonSchemaChange,
  onRecipeNameChange,
  onGlobalChange,
}: RecipeFormFieldsProps) {
  const [showJsonSchemaEditor, setShowJsonSchemaEditor] = useState(false);
  const [showInstructionsEditor, setShowInstructionsEditor] = useState(false);
  const [newParameterName, setNewParameterName] = useState('');
  const [expandedParameters, setExpandedParameters] = useState<Set<string>>(new Set());

  const parseParametersFromInstructions = React.useCallback(
    (instructions: string, prompt?: string, activities?: string[]): Parameter[] => {
      const instructionVars = extractTemplateVariables(instructions);
      const promptVars = prompt ? extractTemplateVariables(prompt) : [];
      const activityVars = activities
        ? activities.flatMap((activity) => extractTemplateVariables(activity))
        : [];

      // Combine and deduplicate
      const allVars = [...new Set([...instructionVars, ...promptVars, ...activityVars])];

      return allVars.map((key: string) => ({
        key,
        description: `Enter value for ${key}`,
        requirement: 'required' as const,
        input_type: 'string' as const,
      }));
    },
    []
  );

  // Function to update parameters based on current field values
  const updateParametersFromFields = React.useCallback(() => {
    const currentValues = form.state.values;
    const { instructions, prompt, activities, parameters: currentParams } = currentValues;

    const newParams = parseParametersFromInstructions(instructions, prompt, activities);

    // Separate manually added parameters (those not found in instructions/prompt/activities)
    const manualParams = currentParams.filter((param: Parameter) => {
      // Only keep manual params that have a valid key and are not found in the parsed params
      return (
        param.key && param.key.trim() && !newParams.some((newParam) => newParam.key === param.key)
      );
    });

    // Combine parsed parameters with manually added ones, filtering out empty ones
    const combinedParams = [
      ...newParams.map((newParam) => {
        const existing = currentParams.find((cp: Parameter) => cp.key === newParam.key);
        return existing ? { ...existing } : newParam;
      }),
      ...manualParams,
    ].filter((param: Parameter) => param.key && param.key.trim()) as Parameter[];

    // Only update if parameters actually changed
    const currentParamKeys = currentParams.map((p: Parameter) => p.key).sort();
    const newParamKeys = combinedParams.map((p) => p.key).sort();

    if (JSON.stringify(currentParamKeys) !== JSON.stringify(newParamKeys)) {
      form.setFieldValue('parameters', combinedParams);
    }
  }, [form, parseParametersFromInstructions]);

  const isParameterUsed = (
    paramKey: string,
    instructions: string,
    prompt?: string,
    activities?: string[]
  ): boolean => {
    const regex = new RegExp(
      `\\{\\{\\s*${paramKey.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}\\s*\\}\\}`,
      'g'
    );
    const usedInInstructions = regex.test(instructions);
    const usedInPrompt = prompt ? regex.test(prompt) : false;
    const usedInActivities = activities
      ? activities.some((activity) => {
          // For activities, we need to check the full activity string, including message: prefixes
          return regex.test(activity);
        })
      : false;
    return usedInInstructions || usedInPrompt || usedInActivities;
  };

  return (
    <div className="space-y-4" data-testid="recipe-form">
      {/* Title Field */}
      <form.Field name="title">
        {(field: FormFieldApi<string>) => (
          <div>
            <label
              htmlFor="recipe-title"
              className="block text-sm font-medium text-text-standard mb-2"
            >
              Title <span className="text-red-500">*</span>
            </label>
            <input
              id="recipe-title"
              type="text"
              value={field.state.value}
              onChange={(e) => {
                field.handleChange(e.target.value);
                onTitleChange?.(e.target.value);
              }}
              onBlur={field.handleBlur}
              className={`w-full p-3 border rounded-lg bg-background-default text-text-standard focus:outline-none focus:ring-2 focus:ring-blue-500 ${
                field.state.meta.errors.length > 0 ? 'border-red-500' : 'border-border-subtle'
              }`}
              placeholder="Recipe title"
              data-testid="title-input"
            />
            {field.state.meta.errors.length > 0 && (
              <p className="text-red-500 text-sm mt-1">{field.state.meta.errors[0]}</p>
            )}
          </div>
        )}
      </form.Field>

      {/* Description Field */}
      <form.Field name="description">
        {(field: FormFieldApi<string>) => (
          <div>
            <label
              htmlFor="recipe-description"
              className="block text-sm font-medium text-text-standard mb-2"
            >
              Description <span className="text-red-500">*</span>
            </label>
            <input
              id="recipe-description"
              type="text"
              value={field.state.value}
              onChange={(e) => {
                field.handleChange(e.target.value);
                onDescriptionChange?.(e.target.value);
              }}
              onBlur={field.handleBlur}
              className={`w-full p-3 border rounded-lg bg-background-default text-text-standard focus:outline-none focus:ring-2 focus:ring-blue-500 ${
                field.state.meta.errors.length > 0 ? 'border-red-500' : 'border-border-subtle'
              }`}
              placeholder="Brief description of what this recipe does"
              data-testid="description-input"
            />
            {field.state.meta.errors.length > 0 && (
              <p className="text-red-500 text-sm mt-1">{field.state.meta.errors[0]}</p>
            )}
          </div>
        )}
      </form.Field>

      {/* Instructions Field */}
      <form.Field name="instructions">
        {(field: FormFieldApi<string>) => (
          <div>
            <div className="flex items-center justify-between mb-2">
              <label
                htmlFor="recipe-instructions"
                className="block text-sm font-medium text-text-standard"
              >
                Instructions <span className="text-red-500">*</span>
              </label>
              <Button
                type="button"
                onClick={() => setShowInstructionsEditor(true)}
                variant="outline"
                size="sm"
                className="text-xs"
              >
                Open Editor
              </Button>
            </div>
            <textarea
              id="recipe-instructions"
              value={field.state.value}
              onChange={(e) => {
                field.handleChange(e.target.value);
                onInstructionsChange?.(e.target.value);
              }}
              onBlur={() => {
                field.handleBlur();
                updateParametersFromFields();
              }}
              className={`w-full p-3 border rounded-lg bg-background-default text-text-standard focus:outline-none focus:ring-2 focus:ring-blue-500 resize-none font-mono text-sm ${
                field.state.meta.errors.length > 0 ? 'border-red-500' : 'border-border-subtle'
              }`}
              placeholder="Detailed instructions for the AI, hidden from the user..."
              rows={8}
              data-testid="instructions-input"
            />
            <p className="text-xs text-text-muted mt-1">
              Use {`{{parameter_name}}`} to define parameters that users can fill in
            </p>
            {field.state.meta.errors.length > 0 && (
              <p className="text-red-500 text-sm mt-1">{field.state.meta.errors[0]}</p>
            )}

            {/* Instructions Editor Modal */}
            <InstructionsEditor
              isOpen={showInstructionsEditor}
              onClose={() => setShowInstructionsEditor(false)}
              value={field.state.value}
              onChange={(value) => {
                field.handleChange(value);
                onInstructionsChange?.(value);
                updateParametersFromFields();
              }}
              error={field.state.meta.errors.length > 0 ? field.state.meta.errors[0] : undefined}
            />
          </div>
        )}
      </form.Field>

      {/* Initial Prompt Field */}
      <form.Field name="prompt">
        {(field: FormFieldApi<string | undefined>) => (
          <div>
            <label
              htmlFor="recipe-prompt"
              className="block text-sm font-medium text-text-standard mb-2"
            >
              Initial Prompt
            </label>
            <p className="text-xs text-text-muted mt-2 mb-2">
              (Optional - Instructions or Prompt are required)
            </p>
            <textarea
              id="recipe-prompt"
              value={field.state.value || ''}
              onChange={(e) => {
                field.handleChange(e.target.value);
                onPromptChange?.(e.target.value);
              }}
              onBlur={() => {
                field.handleBlur();
                updateParametersFromFields();
              }}
              className="w-full p-3 border border-border-subtle rounded-lg bg-background-default text-text-standard focus:outline-none focus:ring-2 focus:ring-blue-500 resize-none"
              placeholder="Pre-filled prompt when the recipe starts..."
              rows={3}
              data-testid="prompt-input"
            />
          </div>
        )}
      </form.Field>

      {/* Activities Field */}
      <form.Field name="activities">
        {(field: FormFieldApi<string[]>) => (
          <div>
            <RecipeActivityEditor
              activities={field.state.value}
              setActivities={(activities) => field.handleChange(activities)}
              onBlur={updateParametersFromFields}
            />
          </div>
        )}
      </form.Field>

      {/* Parameters Field */}
      <form.Field name="parameters">
        {(field: FormFieldApi<Parameter[]>) => {
          const handleAddParameter = () => {
            if (newParameterName.trim()) {
              const newParam: Parameter = {
                key: newParameterName.trim(),
                description: `Enter value for ${newParameterName.trim()}`,
                input_type: 'string',
                requirement: 'required',
              };
              field.handleChange([...field.state.value, newParam]);
              setNewParameterName('');
              // Expand the newly added parameter by default
              setExpandedParameters((prev) => {
                const newSet = new Set(prev);
                newSet.add(newParam.key);
                return newSet;
              });
            }
          };

          const handleKeyPress = (e: React.KeyboardEvent) => {
            if (e.key === 'Enter') {
              e.preventDefault();
              handleAddParameter();
            }
          };

          const handleDeleteParameter = (parameterKey: string) => {
            const updatedParams = field.state.value.filter(
              (param: Parameter) => param.key !== parameterKey
            );
            field.handleChange(updatedParams);
            // Remove from expanded set if it was expanded
            setExpandedParameters((prev) => {
              const newSet = new Set(prev);
              newSet.delete(parameterKey);
              return newSet;
            });
          };

          const handleToggleExpanded = (parameterKey: string) => {
            setExpandedParameters((prev) => {
              const newSet = new Set(prev);
              if (newSet.has(parameterKey)) {
                newSet.delete(parameterKey);
              } else {
                newSet.add(parameterKey);
              }
              return newSet;
            });
          };

          return (
            <div>
              <label className="block text-md text-textProminent mb-2 font-bold">Parameters</label>
              <p className="text-textSubtle text-sm space-y-2 pb-4">
                Parameters will be automatically detected from {`{{parameter_name}}`} syntax in
                instructions/prompt/activities or you can manually add them below.
              </p>

              {/* Add Parameter Input - Always Visible */}
              <div className="flex gap-2 mb-4">
                <input
                  type="text"
                  value={newParameterName}
                  onChange={(e) => setNewParameterName(e.target.value)}
                  onKeyPress={handleKeyPress}
                  placeholder="Enter parameter name..."
                  className="flex-1 px-3 py-2 border border-border-subtle rounded-lg bg-background-default text-text-standard focus:outline-none focus:ring-2 focus:ring-blue-500 text-sm"
                />
                <button
                  type="button"
                  onClick={handleAddParameter}
                  disabled={!newParameterName.trim()}
                  className="px-4 py-2 bg-blue-500 text-white rounded-lg text-sm hover:bg-blue-600 transition-colors disabled:bg-gray-400 disabled:cursor-not-allowed"
                >
                  Add parameter
                </button>
              </div>

              {field.state.value.length > 0 &&
                field.state.value
                  .filter((parameter: Parameter) => parameter.key && parameter.key.trim()) // Filter out empty parameters
                  .map((parameter: Parameter) => {
                    const currentValues = form.state.values;
                    const isUnused = !isParameterUsed(
                      parameter.key,
                      currentValues.instructions,
                      currentValues.prompt,
                      currentValues.activities
                    );

                    return (
                      <ParameterInput
                        key={parameter.key}
                        parameter={parameter}
                        isUnused={isUnused}
                        isExpanded={expandedParameters.has(parameter.key)}
                        onToggleExpanded={handleToggleExpanded}
                        onDelete={handleDeleteParameter}
                        onChange={(name, value) => {
                          const updatedParams = field.state.value.map((param: Parameter) =>
                            param.key === name ? { ...param, ...value } : param
                          );
                          field.handleChange(updatedParams);
                        }}
                      />
                    );
                  })}
            </div>
          );
        }}
      </form.Field>

      {/* JSON Schema Field */}
      <form.Field name="jsonSchema">
        {(field: FormFieldApi<string | undefined>) => (
          <div>
            <label className="block text-md text-textProminent mb-2 font-bold">
              Response JSON Schema
            </label>
            <p className="text-textSubtle text-sm space-y-2 pb-4">
              Define the expected structure of the AI's response using JSON Schema format
            </p>
            <div className="flex items-center justify-between mb-2">
              <Button
                type="button"
                onClick={() => setShowJsonSchemaEditor(true)}
                variant="outline"
                size="sm"
                className="text-xs"
              >
                Open Editor
              </Button>
            </div>

            {field.state.value && field.state.value.trim() && (
              <div
                className={`border rounded-lg p-3 bg-background-muted ${
                  field.state.meta.errors.length > 0 ? 'border-red-500' : 'border-border-subtle'
                }`}
              >
                <pre className="text-xs font-mono text-text-standard whitespace-pre-wrap break-words max-h-32 overflow-y-auto">
                  {field.state.value}
                </pre>
              </div>
            )}

            {field.state.meta.errors.length > 0 && (
              <p className="text-red-500 text-sm mt-1">{field.state.meta.errors[0]}</p>
            )}

            {/* JSON Schema Editor Modal */}
            <JsonSchemaEditor
              isOpen={showJsonSchemaEditor}
              onClose={() => setShowJsonSchemaEditor(false)}
              value={field.state.value || ''}
              onChange={(value) => {
                field.handleChange(value);
                onJsonSchemaChange?.(value);
              }}
              error={field.state.meta.errors.length > 0 ? field.state.meta.errors[0] : undefined}
            />
          </div>
        )}
      </form.Field>

      {/* Recipe Name Field */}
      <form.Field name="recipeName">
        {(field: FormFieldApi<string | undefined>) => (
          <div>
            <div data-testid="recipe-name-field">
              <RecipeNameField
                id="recipe-name-field"
                value={field.state.value || ''}
                onChange={(value) => {
                  field.handleChange(value);
                  onRecipeNameChange?.(value);
                }}
                onBlur={field.handleBlur}
                errors={field.state.meta.errors}
              />
            </div>
          </div>
        )}
      </form.Field>

      {/* Save Location Field */}
      <form.Field name="global">
        {(field: FormFieldApi<boolean>) => (
          <div data-testid="save-location-field">
            <label className="block text-sm font-medium text-text-standard mb-2">
              Save Location
            </label>
            <div className="space-y-2">
              <label className="flex items-center">
                <input
                  type="radio"
                  name="save-location"
                  checked={field.state.value === true}
                  onChange={() => {
                    field.handleChange(true);
                    onGlobalChange?.(true);
                  }}
                  className="mr-2"
                  data-testid="global-radio"
                />
                <span className="text-sm text-text-standard">
                  Global - Available across all Goose sessions
                </span>
              </label>
              <label className="flex items-center">
                <input
                  type="radio"
                  name="save-location"
                  checked={field.state.value === false}
                  onChange={() => {
                    field.handleChange(false);
                    onGlobalChange?.(false);
                  }}
                  className="mr-2"
                  data-testid="directory-radio"
                />
                <span className="text-sm text-text-standard">
                  Directory - Available in the working directory
                </span>
              </label>
            </div>
          </div>
        )}
      </form.Field>
    </div>
  );
}
