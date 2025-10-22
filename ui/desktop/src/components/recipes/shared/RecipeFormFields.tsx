import React, { useState } from 'react';
import { Parameter } from '../../../recipe';
import { ChevronDown, ChevronUp } from 'lucide-react';

import ParameterInput from '../../parameter/ParameterInput';
import JsonSchemaEditor from './JsonSchemaEditor';
import InstructionsEditor from './InstructionsEditor';
import { Button } from '../../ui/button';
import { RecipeFormApi } from './recipeFormSchema';
import { ScheduleConfigSection, ScheduleConfig } from '../../shared/ScheduleConfigSection';

// Type for field API to avoid linting issues
// eslint-disable-next-line @typescript-eslint/no-explicit-any
type FormFieldApi<_T = any> = any;

interface RecipeFormFieldsProps {
  form: RecipeFormApi;
  onTitleChange?: (value: string) => void;
  onDescriptionChange?: (value: string) => void;
  onInstructionsChange?: (value: string) => void;
  onPromptChange?: (value: string) => void;
  onJsonSchemaChange?: (value: string) => void;
  // Schedule configuration props
  recipeTitle?: string;
  scheduleConfig?: ScheduleConfig;
  onScheduleConfigChange?: (config: ScheduleConfig | null) => void;
}

export const extractTemplateVariables = (content: string): string[] => {
  const templateVarRegex = /\{\{(.*?)\}\}/g;
  const variables: string[] = [];
  let match;

  while ((match = templateVarRegex.exec(content)) !== null) {
    const variable = match[1].trim();
    if (variable && !variables.includes(variable)) {
      const validVarRegex = /^\s*[a-zA-Z_][a-zA-Z0-9_]*\s*$/;
      if (validVarRegex.test(variable)) {
        variables.push(variable);
      }
    }
  }
  return variables;
};

export function RecipeFormFields({
  form,
  onTitleChange,
  onDescriptionChange,
  onInstructionsChange,
  onPromptChange,
  onJsonSchemaChange,
  recipeTitle,
  scheduleConfig,
  onScheduleConfigChange,
}: RecipeFormFieldsProps) {
  // Advanced configuration state
  const [showAdvanced, setShowAdvanced] = useState(false);
  
  // Other states
  const [showInstructionsEditor, setShowInstructionsEditor] = useState(false);
  const [newParameterName, setNewParameterName] = useState('');
  const [expandedParameters, setExpandedParameters] = useState<Set<string>>(new Set());
  const [_forceRender, setForceRender] = useState(0);

  React.useEffect(() => {
    return form.store.subscribe(() => {
      setForceRender((prev) => prev + 1);
    });
  }, [form.store]);

  const parseParametersFromInstructions = React.useCallback(
    (instructions: string, prompt?: string): Parameter[] => {
      const instructionVars = extractTemplateVariables(instructions);
      const promptVars = prompt ? extractTemplateVariables(prompt) : [];
      const allVars = [...new Set([...instructionVars, ...promptVars])];

      return allVars.map((key: string) => ({
        key,
        description: `Enter value for ${key}`,
        requirement: 'required' as const,
        input_type: 'string' as const,
      }));
    },
    []
  );

  const updateParametersFromFields = React.useCallback(() => {
    const currentValues = form.state.values;
    const { instructions, prompt, parameters: currentParams } = currentValues;

    const newParams = parseParametersFromInstructions(instructions, prompt);
    const manualParams = currentParams.filter((param: Parameter) => {
      return !newParams.some((np) => np.key === param.key);
    });

    const allParams = [...newParams, ...manualParams];
    form.setFieldValue('parameters', allParams);
  }, [form, parseParametersFromInstructions]);

  React.useEffect(() => {
    updateParametersFromFields();
  }, [updateParametersFromFields]);

  return (
    <div className="space-y-4">
      {/* REQUIRED: Title Field */}
      <form.Field name="title">
        {(field: FormFieldApi<string>) => (
          <div>
            <label htmlFor="recipe-title" className="block text-sm font-medium text-text-standard mb-2">
              Title <span className="text-red-500">*</span>
            </label>
            <input
              id="recipe-title"
              type="text"
              value={field.state.value || ''}
              onChange={(e) => {
                field.handleChange(e.target.value);
                onTitleChange?.(e.target.value);
              }}
              onBlur={field.handleBlur}
              className={`w-full p-3 border rounded-lg bg-background-default text-text-standard focus:outline-none focus:ring-2 focus:ring-blue-500 ${
                field.state.meta.errors.length > 0 ? 'border-red-500' : 'border-border-subtle'
              }`}
              placeholder="Give your command a descriptive name"
              data-testid="title-input"
            />
            {field.state.meta.errors.length > 0 && (
              <p className="text-red-500 text-sm mt-1">{field.state.meta.errors[0]}</p>
            )}
          </div>
        )}
      </form.Field>

      {/* REQUIRED: Instructions Field */}
      <form.Field name="instructions">
        {(field: FormFieldApi<string>) => (
          <div>
            <div className="flex items-center justify-between mb-2">
              <label htmlFor="recipe-instructions" className="block text-sm font-medium text-text-standard">
                Instructions <span className="text-red-500">*</span>
              </label>
              <Button
                type="button"
                onClick={() => setShowInstructionsEditor(!showInstructionsEditor)}
                variant="ghost"
                size="sm"
                className="text-xs"
              >
                {showInstructionsEditor ? 'Hide' : 'Show'} Editor
              </Button>
            </div>

            {showInstructionsEditor ? (
              <InstructionsEditor
                value={field.state.value || ''}
                onChange={(value) => {
                  field.handleChange(value);
                  onInstructionsChange?.(value);
                }}
              />
            ) : (
              <textarea
                id="recipe-instructions"
                value={field.state.value || ''}
                onChange={(e) => {
                  field.handleChange(e.target.value);
                  onInstructionsChange?.(e.target.value);
                }}
                onBlur={field.handleBlur}
                className={`w-full p-3 border rounded-lg bg-background-default text-text-standard focus:outline-none focus:ring-2 focus:ring-blue-500 resize-none font-mono text-sm ${
                  field.state.meta.errors.length > 0 ? 'border-red-500' : 'border-border-subtle'
                }`}
                placeholder="Detailed instructions for the AI, hidden from the user..."
                rows={8}
                data-testid="instructions-input"
              />
            )}
            {field.state.meta.errors.length > 0 && (
              <p className="text-red-500 text-sm mt-1">{field.state.meta.errors[0]}</p>
            )}
            <p className="text-text-muted text-xs mt-1">
              Use {`{{variable_name}}`} syntax to create parameters
            </p>
          </div>
        )}
      </form.Field>

      {/* SCHEDULE CONFIGURATION - Moved above Advanced Configuration */}
      {onScheduleConfigChange && (
        <ScheduleConfigSection
          recipeTitle={recipeTitle}
          value={scheduleConfig}
          onChange={onScheduleConfigChange}
        />
      )}

      {/* ADVANCED CONFIGURATION - Collapsible Section */}
      <div className="border border-border-subtle rounded-lg p-4">
        <button
          type="button"
          onClick={() => setShowAdvanced(!showAdvanced)}
          className="w-full flex items-center justify-between"
        >
          <div className="flex items-center gap-2">
            <h3 className="text-base font-semibold text-text-prominent">
              Advanced Configuration
            </h3>
            <span className="text-xs text-text-muted">(Optional)</span>
          </div>
          {showAdvanced ? (
            <ChevronUp className="w-5 h-5 text-text-muted" />
          ) : (
            <ChevronDown className="w-5 h-5 text-text-muted" />
          )}
        </button>

        {showAdvanced && (
          <div className="mt-6 space-y-6 animate-in fade-in slide-in-from-top-2 duration-200">
            {/* Description Field */}
            <form.Field name="description">
              {(field: FormFieldApi<string>) => (
                <div>
                  <label htmlFor="recipe-description" className="block text-sm font-medium text-text-standard mb-2">
                    Description
                  </label>
                  <textarea
                    id="recipe-description"
                    value={field.state.value || ''}
                    onChange={(e) => {
                      field.handleChange(e.target.value);
                      onDescriptionChange?.(e.target.value);
                    }}
                    onBlur={field.handleBlur}
                    className={`w-full p-3 border rounded-lg bg-background-default text-text-standard focus:outline-none focus:ring-2 focus:ring-blue-500 resize-none ${
                      field.state.meta.errors.length > 0 ? 'border-red-500' : 'border-border-subtle'
                    }`}
                    placeholder="Brief description of what this command does"
                    rows={3}
                    data-testid="description-input"
                  />
                  {field.state.meta.errors.length > 0 && (
                    <p className="text-red-500 text-sm mt-1">{field.state.meta.errors[0]}</p>
                  )}
                </div>
              )}
            </form.Field>

            {/* Initial Prompt Field */}
            <form.Field name="prompt">
              {(field: FormFieldApi<string | undefined>) => (
                <div>
                  <label htmlFor="recipe-prompt" className="block text-sm font-medium text-text-standard mb-2">
                    Initial Prompt
                  </label>
                  <textarea
                    id="recipe-prompt"
                    value={field.state.value || ''}
                    onChange={(e) => {
                      field.handleChange(e.target.value);
                      onPromptChange?.(e.target.value);
                    }}
                    onBlur={field.handleBlur}
                    className="w-full p-3 border border-border-subtle rounded-lg bg-background-default text-text-standard focus:outline-none focus:ring-2 focus:ring-blue-500 resize-none"
                    placeholder="Pre-filled prompt when the command starts..."
                    rows={3}
                    data-testid="prompt-input"
                  />
                  <p className="text-text-muted text-xs mt-1">
                    This message will appear in the chat when the command starts
                  </p>
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
                      requirement: 'required',
                      input_type: 'string',
                    };
                    field.handleChange([...field.state.value, newParam]);
                    setNewParameterName('');
                  }
                };

                const handleRemoveParameter = (index: number) => {
                  const updated = field.state.value.filter((_: Parameter, i: number) => i !== index);
                  field.handleChange(updated);
                };

                const handleUpdateParameter = (index: number, updated: Parameter) => {
                  const newParams = [...field.state.value];
                  newParams[index] = updated;
                  field.handleChange(newParams);
                };

                const toggleExpanded = (key: string) => {
                  const newExpanded = new Set(expandedParameters);
                  if (newExpanded.has(key)) {
                    newExpanded.delete(key);
                  } else {
                    newExpanded.add(key);
                  }
                  setExpandedParameters(newExpanded);
                };

                const handleKeyPress = (e: React.KeyboardEvent<HTMLInputElement>) => {
                  if (e.key === 'Enter') {
                    e.preventDefault();
                    handleAddParameter();
                  }
                };

                return (
                  <div>
                    <label className="block text-sm font-medium text-text-standard mb-2">
                      Parameters
                    </label>
                    <p className="text-text-muted text-sm mb-4">
                      Parameters will be automatically detected from {`{{parameter_name}}`} syntax in
                      instructions/prompt or you can manually add them below.
                    </p>

                    {/* Add parameter input */}
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
                        className="px-4 py-2 bg-blue-500 text-white rounded-lg hover:bg-blue-600 disabled:bg-gray-300 disabled:cursor-not-allowed text-sm font-medium transition-colors"
                      >
                        Add
                      </button>
                    </div>

                    {/* Parameters list */}
                    {field.state.value.length > 0 && (
                      <div className="space-y-2">
                        {field.state.value.map((param: Parameter, index: number) => (
                          <ParameterInput
                            key={param.key}
                            parameter={param}
                            isExpanded={expandedParameters.has(param.key)}
                            onToggleExpand={() => toggleExpanded(param.key)}
                            onUpdate={(updated) => handleUpdateParameter(index, updated)}
                            onRemove={() => handleRemoveParameter(index)}
                          />
                        ))}
                      </div>
                    )}
                  </div>
                );
              }}
            </form.Field>

            {/* JSON Schema Field */}
            <form.Field name="jsonSchema">
              {(field: FormFieldApi<string | undefined>) => (
                <div>
                  <label className="block text-sm font-medium text-text-standard mb-2">
                    Response JSON Schema
                  </label>
                  <p className="text-text-muted text-sm mb-4">
                    Define a JSON schema to structure the AI's response format.
                  </p>
                  <JsonSchemaEditor
                    value={field.state.value || ''}
                    onChange={(value) => {
                      field.handleChange(value);
                      onJsonSchemaChange?.(value);
                    }}
                  />
                </div>
              )}
            </form.Field>
          </div>
        )}
      </div>
    </div>
  );
}
