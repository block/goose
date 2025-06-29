import React, { useState, useEffect } from 'react';

// Assuming Parameter type is defined somewhere accessible, like in your recipe types
interface Parameter {
  name: string;
  promptMessage: string;
  defaultValue?: string;
  requirement: 'required' | 'optional' | 'interactive';
}

const ParameterInputModal = ({ parameters, onSubmit, onClose }) => {
    const [inputValues, setInputValues] = useState({});

    // Pre-fill the form with default values from the recipe
    useEffect(() => {
        const initialValues = {};
        parameters.forEach(param => {
            if (param.defaultValue) {
                initialValues[param.name] = param.defaultValue;
            }
        });
        setInputValues(initialValues);
    }, [parameters]);

    const handleChange = (name, value) => {
        setInputValues(prevValues => ({ ...prevValues, [name]: value }));
    };

    const handleSubmit = (event) => {
        event.preventDefault();
        // Check if all *required* parameters are filled
        const requiredParams = parameters.filter(p => p.requirement === 'required');
        const missingFields = requiredParams.filter(p => !inputValues[p.name]?.trim());

        if (missingFields.length > 0) {
            alert(`Please fill in all required fields: ${missingFields.map(p => p.name).join(', ')}`);
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
                    {parameters.map(param => (
                        <div key={param.name}>
                            <label className="block text-md font-medium text-textStandard mb-2">
                                {param.promptMessage || param.name}
                                {param.requirement === 'required' && <span className="text-red-500 ml-1">*</span>}
                            </label>
                            <input
                                type="text"
                                value={inputValues[param.name] || ''}
                                onChange={(e) => handleChange(param.name, e.target.value)}
                                className="w-full p-3 border border-borderSubtle rounded-lg bg-bgSubtle text-textStandard focus:outline-none focus:ring-2 focus:ring-borderProminent"
                                placeholder={param.defaultValue || `Enter value for ${param.name}...`}
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