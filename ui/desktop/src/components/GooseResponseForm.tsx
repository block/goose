import React, { useState, useEffect, useRef } from 'react';
import MarkdownContent from './MarkdownContent';
import { Button } from './ui/button';
import { cn } from '../utils';
import { Send } from './icons';
// Prefixing unused imports with underscore
import { createUserMessage as _createUserMessage } from '../types/message';

interface FormField {
  label: string;
  type: 'text' | 'textarea';
  name: string;
  placeholder: string;
  required: boolean;
}

interface DynamicForm {
  title: string;
  description: string;
  fields: FormField[];
}

interface GooseResponseFormProps {
  message: string;
  metadata: string[] | null;
  append: (value: string) => void;
}

export default function GooseResponseForm({
  message: _message,
  metadata,
  append,
}: GooseResponseFormProps) {
  const [selectedOption, setSelectedOption] = useState<number | null>(null);
  const [formValues, setFormValues] = useState<Record<string, string>>({});
  const prevStatusRef = useRef<string | null>(null);

  let isQuestion = false;
  let isOptions = false;
  let options: Array<{ optionTitle: string; optionDescription: string }> = [];
  let dynamicForm: DynamicForm | null = null;

  if (metadata) {
    window.electron.logInfo('metadata:' + JSON.stringify(metadata, null, 2));
  }

  // Process metadata outside of conditional
  const currentStatus = metadata?.[0] ?? null;
  isQuestion = currentStatus === 'QUESTION';
  isOptions = metadata?.[1] === 'OPTIONS';

  // Parse dynamic form data if it exists in metadata[3]
  if (metadata?.[3]) {
    try {
      dynamicForm = JSON.parse(metadata[3]);
    } catch (err) {
      console.error('Failed to parse form data:', err);
      dynamicForm = null;
    }
  }

  if (isQuestion && isOptions && metadata?.[2]) {
    try {
      let optionsData = metadata[2];
      // Use a regular expression to extract the JSON block
      const jsonBlockMatch = optionsData.match(/```json([\s\S]*?)```/);

      // If a JSON block is found, extract and clean it
      if (jsonBlockMatch) {
        optionsData = jsonBlockMatch[1].trim(); // Extract the content inside the block
      } else {
        // Optionally, handle the case where there is no explicit ```json block
        console.warn('No JSON block found in the provided string.');
      }
      options = JSON.parse(optionsData);
      options = options.filter(
        (opt) => typeof opt.optionTitle === 'string' && typeof opt.optionDescription === 'string'
      );
    } catch (err) {
      console.error('Failed to parse options data:', err);
      options = [];
    }
  }

  // Move useEffect to top level
  useEffect(() => {
    const currentMetadataStatus = metadata?.[0];
    const shouldNotify =
      currentMetadataStatus &&
      (currentMetadataStatus === 'QUESTION' || currentMetadataStatus === 'OPTIONS') &&
      prevStatusRef.current !== currentMetadataStatus;

    if (shouldNotify) {
      window.electron.showNotification({
        title: 'Goose has a question for you',
        body: `Please check with Goose to approve the plan of action`,
      });
    }

    prevStatusRef.current = currentMetadataStatus ?? null;
  }, [metadata]);

  const handleOptionClick = (index: number) => {
    setSelectedOption(index);
  };

  const handleAccept = () => {
    append('Yes - go ahead.');
  };

  const handleSubmit = () => {
    if (selectedOption !== null && options[selectedOption]) {
      append(`Yes - continue with: ${options[selectedOption].optionTitle}`);
    }
  };

  const handleFormSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (dynamicForm) {
      append(JSON.stringify(formValues));
    }
  };

  const handleFormChange = (name: string, value: string) => {
    setFormValues((prev) => ({
      ...prev,
      [name]: value,
    }));
  };

  if (!metadata) {
    return null;
  }

  function isForm(f: DynamicForm | null): f is DynamicForm {
    return (
      !!f &&
      !!f.title &&
      !!f.description &&
      !!f.fields &&
      Array.isArray(f.fields) &&
      f.fields.length > 0
    );
  }

  return (
    <div className="space-y-4">
      {isQuestion && !isOptions && !isForm(dynamicForm) && (
        <div className="flex items-center gap-4 p-4 rounded-lg bg-tool-card dark:bg-tool-card-dark border dark:border-dark-border">
          <Button onClick={handleAccept} className="w-full sm:w-auto">
            <Send className="h-[14px] w-[14px]" />
            Take flight with this plan
          </Button>
        </div>
      )}
      {isQuestion && isOptions && Array.isArray(options) && options.length > 0 && (
        <div className="space-y-4">
          {options.map((opt, index) => (
            <div
              key={index}
              onClick={() => handleOptionClick(index)}
              className={cn(
                'p-4 rounded-lg border transition-colors cursor-pointer',
                selectedOption === index
                  ? 'bg-primary/10 dark:bg-dark-primary border-primary dark:border-dark-primary'
                  : 'bg-tool-card dark:bg-tool-card-dark hover:bg-accent dark:hover:bg-dark-accent'
              )}
            >
              <h3 className="font-semibold text-lg mb-2 dark:text-gray-100">{opt.optionTitle}</h3>
              <div className="prose prose-xs max-w-none dark:text-gray-100">
                <MarkdownContent content={opt.optionDescription} />
              </div>
            </div>
          ))}
          <Button
            onClick={handleSubmit}
            variant="default"
            className="w-full sm:w-auto dark:bg-button-dark"
            disabled={selectedOption === null}
          >
            <Send className="h-[14px] w-[14px]" />
            Select plan
          </Button>
        </div>
      )}
      {isForm(dynamicForm) && !isOptions && (
        <form
          onSubmit={handleFormSubmit}
          className="space-y-4 p-4 rounded-lg bg-tool-card dark:bg-tool-card-dark border dark:border-dark-border"
        >
          <h2 className="text-xl font-medium mb-2 dark:text-gray-100">{dynamicForm.title}</h2>
          <p className="text-sm text-gray-600 dark:text-gray-300 mb-4">{dynamicForm.description}</p>

          {dynamicForm.fields.map((field) => (
            <div key={field.name} className="space-y-2">
              <label
                htmlFor={field.name}
                className="block text-sm font-medium text-gray-700 dark:text-gray-200"
              >
                {field.label}
                {field.required && <span className="text-red-500 ml-1">*</span>}
              </label>
              {field.type === 'textarea' ? (
                <textarea
                  id={field.name}
                  name={field.name}
                  placeholder={field.placeholder}
                  required={field.required}
                  value={formValues[field.name] || ''}
                  onChange={(e) => handleFormChange(field.name, e.target.value)}
                  className="w-full p-2 border rounded-md dark:bg-gray-700 dark:border-gray-600 dark:text-gray-100"
                  rows={4}
                />
              ) : (
                <input
                  type="text"
                  id={field.name}
                  name={field.name}
                  placeholder={field.placeholder}
                  required={field.required}
                  value={formValues[field.name] || ''}
                  onChange={(e) => handleFormChange(field.name, e.target.value)}
                  className="w-full p-2 border rounded-md dark:bg-gray-700 dark:border-gray-600 dark:text-gray-100"
                />
              )}
            </div>
          ))}

          <Button type="submit" className="w-full sm:w-auto mt-4">
            <Send className="h-[14px] w-[14px]" />
            Submit Form
          </Button>
        </form>
      )}
    </div>
  );
}
