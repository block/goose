import React, { useState, useEffect } from 'react';
import { Parameter } from '../recipe';
interface ParameterInputModalProps {
  parameters: Parameter[];
  onSubmit: (values: Record<string, string>) => void;
  onClose: () => void;
}

const ParameterInputModal: React.FC<ParameterInputModalProps> = ({
  parameters,
  onSubmit,
  onClose,
}) => {
  const [inputValues, setInputValues] = useState<Record<string, string>>({});

  // Pre-fill the form with default values from the recipe
  useEffect(() => {
    const initialValues: Record<string, string> = {};
    parameters.forEach((param) => {
      if (param.default) {
        initialValues[param.key] = param.default;
      }
    });
    setInputValues(initialValues);
  }, [parameters]);

  interface MissingField {
    key: string;
    name?: string;
    requirement?: string;
  }

  const handleChange = (name: string, value: string): void => {
    setInputValues((prevValues: Record<string, string>) => ({ ...prevValues, [name]: value }));
  };

  const handleSubmit = (): void => {
    // event.preventDefault();
    // Check if all *required* parameters are filled
    const requiredParams: Parameter[] = parameters.filter((p) => p.requirement === 'required');
    const missingFields: MissingField[] = requiredParams.filter((p) => !inputValues[p.key]?.trim());

    if (missingFields.length > 0) {
      // TODO: Show a user-friendly message instead of an alert
      // alert(`Please fill in all required fields: ${missingFields.map(p => p.name).join(', ')}`);
      return;
    }
    onSubmit(inputValues);
  };

  return (
    // This styling creates a dark, semi-transparent overlay that centers the modal.
    <div className="fixed inset-0 bg-black bg-opacity-60 z-50 flex justify-center items-center animate-[fadein_200ms_ease-in]">
      <div className="bg-bgApp border border-borderSubtle rounded-xl p-8 shadow-2xl w-full max-w-lg">
        <h2 className="text-xl font-bold text-textProminent mb-6">Recipe Parameters</h2>
        <form onSubmit={handleSubmit} className="space-y-4">
          {parameters.map((param) => (
            <div key={param.key}>
              <label className="block text-md font-medium text-textStandard mb-2">
                {param.description || param.key}
                {param.requirement === 'required' && <span className="text-red-500 ml-1">*</span>}
              </label>
              <input
                type="text"
                value={inputValues[param.key] || ''}
                onChange={(e) => handleChange(param.key, e.target.value)}
                className="w-full p-3 border border-borderSubtle rounded-lg bg-bgSubtle text-textStandard focus:outline-none focus:ring-2 focus:ring-borderProminent"
                placeholder={param.default || `Enter value for ${param.key}...`}
              />
            </div>
          ))}
          <div className="flex justify-end gap-4 pt-6">
            <button
              type="button"
              onClick={onClose}
              className="px-4 py-2 rounded-lg text-textStandard hover:bg-bgSubtle border border-borderSubtle transition-colors"
            >
              Cancel
            </button>
            <button
              type="submit"
              className="px-6 py-2 rounded-lg bg-blue-600 text-white font-semibold hover:bg-blue-700 transition-colors"
            >
              Submit
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};

export default ParameterInputModal;
