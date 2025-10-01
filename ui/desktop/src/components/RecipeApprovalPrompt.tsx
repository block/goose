import { useEffect, useMemo, useState } from 'react';
import { Parameter, Recipe } from '../recipe';
import { Card } from './ui/card';
import { Button } from './ui/button';
import MarkdownContent from './MarkdownContent';

interface RecipeApprovalPromptProps {
  recipe: Recipe | null | undefined;
  parameters?: Parameter[];
  existingValues?: Record<string, string> | null;
  requiresTrustApproval?: boolean;
  requiresParameterEntry?: boolean;
  onApprove: (values: Record<string, string>) => Promise<void> | void;
  onReject: () => void;
  isSubmitting?: boolean;
}

export default function RecipeApprovalPrompt({
  recipe,
  parameters = [],
  existingValues,
  requiresTrustApproval = true,
  requiresParameterEntry = false,
  onApprove,
  onReject,
  isSubmitting: externalSubmitting,
}: RecipeApprovalPromptProps) {
  const [internalSubmitting, setInternalSubmitting] = useState(false);
  const [inputValues, setInputValues] = useState<Record<string, string>>({});
  const [validationErrors, setValidationErrors] = useState<Record<string, string>>({});
  const isSubmitting = externalSubmitting ?? internalSubmitting;

  useEffect(() => {
    const initialValues: Record<string, string> = {};
    const prefilledValues = existingValues ?? {};
    parameters.forEach((param) => {
      if (prefilledValues[param.key]) {
        initialValues[param.key] = prefilledValues[param.key];
        return;
      }

      if (param.requirement === 'optional' && param.default) {
        const defaultValue =
          param.input_type === 'boolean' ? param.default.toLowerCase() : param.default;
        initialValues[param.key] = defaultValue;
      }
    });
    setInputValues(initialValues);
  }, [parameters, existingValues]);

  const hasParameters = useMemo(() => parameters.length > 0, [parameters]);

  const handleChange = (key: string, value: string) => {
    setInputValues((prev) => ({ ...prev, [key]: value }));
    setValidationErrors((prev) => {
      if (!prev[key]) {
        return prev;
      }

      const { [key]: _, ...rest } = prev;
      return rest;
    });
  };

  const handleApprove = async () => {
    try {
      if (externalSubmitting === undefined) {
        setInternalSubmitting(true);
      }
      const errors: Record<string, string> = {};
      const requiredParams: Parameter[] = parameters.filter(
        (param) => param.requirement === 'required'
      );

      requiredParams.forEach((param) => {
        const value = inputValues[param.key]?.trim();
        if (!value) {
          errors[param.key] = `${param.description || param.key} is required`;
        }
      });

      if (Object.keys(errors).length > 0) {
        setValidationErrors(errors);
        return;
      }

      await onApprove(inputValues);
    } finally {
      if (externalSubmitting === undefined) {
        setInternalSubmitting(false);
      }
    }
  };

  const title = recipe?.title ?? 'Recipe requires approval';
  const description = recipe?.description;
  const instructions = recipe?.instructions;

  const primaryButtonLabel = useMemo(() => {
    if (requiresTrustApproval && hasParameters) {
      return 'Approve & Start';
    }
    if (requiresTrustApproval) {
      return 'Approve';
    }
    if (hasParameters) {
      return 'Start Recipe';
    }
    return 'Continue';
  }, [requiresTrustApproval, hasParameters]);

  return (
    <Card className="border border-borderSubtle bg-background-default shadow-sm">
      <div className="p-4 space-y-3">
        <div>
          <h2 className="text-lg font-semibold text-textProminent">{title}</h2>
          {description && <p className="text-sm text-textMuted mt-1">{description}</p>}
        </div>

        {requiresTrustApproval && (
          <div className="bg-background-muted border border-borderSubtle rounded-md p-3">
            <p className="text-xs uppercase tracking-wide text-textSubtle mb-2">Approval Required</p>
            <p className="text-sm text-textStandard">
              This recipe has not been approved yet. Review the details and confirm to continue.
            </p>
          </div>
        )}

        {!requiresTrustApproval && requiresParameterEntry && hasParameters && (
          <div className="bg-background-muted border border-borderSubtle rounded-md p-3">
            <p className="text-xs uppercase tracking-wide text-textSubtle mb-2">Parameters Needed</p>
            <p className="text-sm text-textStandard">
              Provide the required parameters below to continue with this recipe.
            </p>
          </div>
        )}

        {instructions && (
          <div className="bg-background-muted border border-borderSubtle rounded-md p-3">
            <p className="text-xs uppercase tracking-wide text-textSubtle mb-2">Instructions</p>
            <MarkdownContent content={instructions} className="prose prose-sm text-textStandard" />
          </div>
        )}

        {parameters.length > 0 && (
          <div className="bg-background-muted border border-borderSubtle rounded-md p-3">
            <p className="text-xs uppercase tracking-wide text-textSubtle mb-4">Parameters</p>
            <div className="space-y-4">
              {parameters.map((param) => {
                const value = inputValues[param.key] ?? '';
                const error = validationErrors[param.key];
                const label = param.description || param.key;

                return (
                  <div key={param.key}>
                    <label className="block text-sm font-medium text-textStandard mb-2">
                      {label}
                      {param.requirement === 'required' && (
                        <span className="text-red-500 ml-1">*</span>
                      )}
                    </label>
                    {param.input_type === 'select' && param.options ? (
                      <select
                        value={value}
                        onChange={(e) => handleChange(param.key, e.target.value)}
                        className={`w-full p-3 border rounded-lg bg-bgSubtle text-textStandard focus:outline-none focus:ring-2 ${
                          error
                            ? 'border-red-500 focus:ring-red-500'
                            : 'border-borderSubtle focus:ring-borderProminent'
                        }`}
                      >
                        <option value="">Select an option...</option>
                        {param.options.map((option) => (
                          <option key={option} value={option}>
                            {option}
                          </option>
                        ))}
                      </select>
                    ) : param.input_type === 'boolean' ? (
                      <select
                        value={value}
                        onChange={(e) => handleChange(param.key, e.target.value)}
                        className={`w-full p-3 border rounded-lg bg-bgSubtle text-textStandard focus:outline-none focus:ring-2 ${
                          error
                            ? 'border-red-500 focus:ring-red-500'
                            : 'border-borderSubtle focus:ring-borderProminent'
                        }`}
                      >
                        <option value="">Select...</option>
                        <option value="true">True</option>
                        <option value="false">False</option>
                      </select>
                    ) : (
                      <input
                        type={param.input_type === 'number' ? 'number' : 'text'}
                        value={value}
                        onChange={(e) => handleChange(param.key, e.target.value)}
                        className={`w-full p-3 border rounded-lg bg-bgSubtle text-textStandard focus:outline-none focus:ring-2 ${
                          error
                            ? 'border-red-500 focus:ring-red-500'
                            : 'border-borderSubtle focus:ring-borderProminent'
                        }`}
                        placeholder={param.default || `Enter value for ${param.key}...`}
                      />
                    )}
                    {error && <p className="text-red-500 text-sm mt-1">{error}</p>}
                  </div>
                );
              })}
            </div>
          </div>
        )}

        <div className="flex justify-end gap-2">
          <Button variant="ghost" onClick={onReject} disabled={isSubmitting}>
            Cancel
          </Button>
          <Button onClick={handleApprove} disabled={isSubmitting}>
            {isSubmitting ? 'Submitting…' : primaryButtonLabel}
          </Button>
        </div>
      </div>
    </Card>
  );
}
