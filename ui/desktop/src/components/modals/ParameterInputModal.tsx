import type React from 'react';
import { useEffect, useState } from 'react';
import type { Parameter } from '../../recipe';
import { getInitialWorkingDir } from '../../utils/workingDir';
import { Button } from '../ui/atoms/button';

interface ParameterInputModalProps {
  parameters: Parameter[];
  onSubmit: (values: Record<string, string>) => void;
  onClose: () => void;
  initialValues?: Record<string, string>;
}

const ParameterInputModal: React.FC<ParameterInputModalProps> = ({
  parameters,
  onSubmit,
  onClose,
  initialValues,
}) => {
  const [inputValues, setInputValues] = useState<Record<string, string>>({});
  const [validationErrors, setValidationErrors] = useState<Record<string, string>>({});
  const [showCancelOptions, setShowCancelOptions] = useState(false);

  // Pre-fill the form with default values from the recipe and initialValues from deeplink
  useEffect(() => {
    const defaultValues: Record<string, string> = {};
    parameters.forEach((param) => {
      if (param.requirement === 'optional' && param.default) {
        defaultValues[param.key] =
          param.input_type === 'boolean' ? param.default.toLowerCase() : param.default;
      }
    });

    setInputValues({ ...defaultValues, ...initialValues });
  }, [parameters, initialValues]);

  const handleChange = (name: string, value: string): void => {
    setInputValues((prevValues: Record<string, string>) => ({ ...prevValues, [name]: value }));
  };

  const handleSubmit = (): void => {
    // Clear previous validation errors
    setValidationErrors({});

    // Check if all *required* parameters are filled
    const requiredParams: Parameter[] = parameters.filter((p) => p.requirement === 'required');
    const errors: Record<string, string> = {};

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

    onSubmit(inputValues);
  };

  const handleCancel = (): void => {
    // Always show cancel options if recipe has any parameters (required or optional)
    const hasAnyParams = parameters.length > 0;

    if (hasAnyParams) {
      setShowCancelOptions(true);
    } else {
      onClose();
    }
  };

  const handleCancelOption = (option: 'new-chat' | 'back-to-form'): void => {
    if (option === 'new-chat') {
      try {
        const workingDir = getInitialWorkingDir();
        window.electron.createChatWindow(undefined, workingDir);
        window.electron.hideWindow();
      } catch (error) {
        console.error('Error creating new window:', error);
        onClose();
      }
    } else {
      setShowCancelOptions(false); // Go back to the parameter form
    }
  };

  return (
    <div className="fixed inset-0 backdrop-blur-sm z-50 flex justify-center items-center animate-[fadein_200ms_ease-in]">
      {showCancelOptions ? (
        // Cancel options modal
        <div className="bg-background-default border border-border-default rounded-xl p-8 shadow-2xl w-full max-w-md">
          <h2 className="text-xl font-bold text-text-default mb-4">Cancel Recipe Setup</h2>
          <p className="text-text-default mb-6">What would you like to do?</p>
          <div className="flex flex-col gap-3">
            <Button
              onClick={() => handleCancelOption('back-to-form')}
              variant="default"
              size="lg"
              className="w-full rounded-full"
            >
              Back to Parameter Form
            </Button>
            <Button
              onClick={() => handleCancelOption('new-chat')}
              variant="outline"
              size="lg"
              className="w-full rounded-full"
            >
              Start New Chat (No Recipe)
            </Button>
          </div>
        </div>
      ) : (
        // Main parameter form
        <div className="bg-background-default border border-border-default rounded-xl shadow-2xl w-full max-w-lg max-h-[90vh] flex flex-col overflow-hidden">
          <div className="p-8 pb-4 flex-shrink-0">
            <h2 className="text-xl font-bold text-text-default mb-6">Recipe Parameters</h2>
          </div>
          <div className="flex-1 overflow-y-auto px-8">
            <form onSubmit={handleSubmit} className="space-y-4 mb-4">
              {parameters.map((param) => (
                <div key={param.key}>
                  <label className="block text-md font-medium text-text-default mb-2">
                    {param.description || param.key}
                    {param.requirement === 'required' && (
                      <span className="text-red-500 ml-1">*</span>
                    )}
                  </label>

                  {/* Render different input types */}
                  {param.input_type === 'select' && param.options ? (
                    <select
                      value={inputValues[param.key] || ''}
                      onChange={(e) => handleChange(param.key, e.target.value)}
                      className={`w-full p-3 border rounded-lg bg-background-muted text-text-default focus:outline-none focus:ring-2 ${
                        validationErrors[param.key]
                          ? 'border-red-500 focus:ring-red-500'
                          : 'border-border-default focus:ring-border-strong'
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
                      value={inputValues[param.key] || ''}
                      onChange={(e) => handleChange(param.key, e.target.value)}
                      className={`w-full p-3 border rounded-lg bg-background-muted text-text-default focus:outline-none focus:ring-2 ${
                        validationErrors[param.key]
                          ? 'border-red-500 focus:ring-red-500'
                          : 'border-border-default focus:ring-border-strong'
                      }`}
                    >
                      <option value="">Select...</option>
                      <option value="true">True</option>
                      <option value="false">False</option>
                    </select>
                  ) : (
                    <input
                      type={param.input_type === 'number' ? 'number' : 'text'}
                      value={inputValues[param.key] || ''}
                      onChange={(e) => handleChange(param.key, e.target.value)}
                      className={`w-full p-3 border rounded-lg bg-background-muted text-text-default focus:outline-none focus:ring-2 ${
                        validationErrors[param.key]
                          ? 'border-red-500 focus:ring-red-500'
                          : 'border-border-default focus:ring-border-strong'
                      }`}
                      placeholder={param.default || `Enter value for ${param.key}...`}
                    />
                  )}

                  {validationErrors[param.key] && (
                    <p className="text-red-500 text-sm mt-1">{validationErrors[param.key]}</p>
                  )}
                </div>
              ))}
            </form>
          </div>
          <div className="p-8 pt-4 flex-shrink-0">
            <div className="flex justify-end gap-4">
              <Button
                type="button"
                onClick={handleCancel}
                variant="outline"
                size="default"
                className="rounded-full"
              >
                Cancel
              </Button>
              <Button
                type="button"
                onClick={handleSubmit}
                variant="default"
                size="default"
                className="rounded-full"
              >
                Start Recipe
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

export default ParameterInputModal;
