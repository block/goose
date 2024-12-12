import React, { useState, useEffect, useRef } from 'react';
import { GPSIcon } from './ui/icons';
import ReactMarkdown from 'react-markdown';
import { Button } from './ui/button';
import { cn } from '../utils';

interface GooseResponseFormProps {
  message: string;
  metadata: any;
  append: (value: any) => void;
}

export default function GooseResponseForm({ message, metadata, append }: GooseResponseFormProps) {
  const [selectedOption, setSelectedOption] = useState(null);
  const prevStatusRef = useRef(null);

  let isQuestion = false;
  let isOptions = false;
  let options = [];

  if (metadata) {
    window.electron.logInfo('metadata:'+ JSON.stringify(metadata, null, 2));
  }

  if (metadata) {
    isQuestion = metadata[0] === "QUESTION";
    isOptions = metadata[1] === "OPTIONS";

    if (isQuestion && isOptions && metadata[2]) {
      try {
        let optionsData = metadata[2];
        // Use a regular expression to extract the JSON block
        const jsonBlockMatch = optionsData.match(/```json([\s\S]*?)```/);

      // If a JSON block is found, extract and clean it
      if (jsonBlockMatch) {
        optionsData = jsonBlockMatch[1].trim(); // Extract the content inside the block
      } else {
        // Optionally, handle the case where there is no explicit ```json block
        console.warn("No JSON block found in the provided string.");
      }
        options = JSON.parse(optionsData);
        options = options.filter(
          (opt) =>
            typeof opt.optionTitle === 'string' &&
            typeof opt.optionDescription === 'string'
        );
      } catch (err) {
        console.error("Failed to parse options data:", err);
        options = [];
      }
    }
  }

  useEffect(() => {
    if (
      (metadata && (metadata[0] === "QUESTION" || metadata[0] === "OPTIONS")) &&
      prevStatusRef.current !== metadata[0]
    ) {
      window.electron.showNotification({
        title: 'Goose has a question for you',
        body: `Please check with Goose to approve the plan of action`,
      });
    }
    prevStatusRef.current = metadata ? metadata[0] : null;
  }, [metadata]);

  const handleOptionClick = (index) => {
    setSelectedOption(index);
  };

  const handleAccept = () => {
    const message = {
      content: "Yes - execute this plan",
      role: "user",
    };
    append(message);
  };

  const handleCancel = () => {
    const message = {
      content: "No - do not execute this plan",
      role: "user",
    };
    append(message);
  };

  const handleSubmit = () => {
    if (selectedOption !== null) {
      const message = {
        content: `Yes - continue with: ${options[selectedOption].optionTitle}`,
        role: "user",
      };
      append(message);
    }
  };

  return (
    <div className="space-y-4">
      {isQuestion && !isOptions && (
        <div className="flex items-center gap-4 p-4 rounded-lg bg-tool-card dark:bg-tool-card-dark border dark:border-dark-border">
          <Button
            onClick={handleAccept}
            variant="default"
            className="w-full sm:w-auto dark:bg-button-dark"
          >
            <GPSIcon size={14} />
            Take flight with this plan
          </Button>
          <Button
            onClick={handleCancel}
            variant="destructive"
            className="w-full sm:w-auto dark:bg-button-destructive-dark"
          >
            <GPSIcon size={14} />
            Cancel
          </Button>
        </div>
      )}
      {isQuestion && isOptions && options.length > 0 && (
        <div className="space-y-4">
          {options.map((opt, index) => (
            <div
              key={index}
              onClick={() => handleOptionClick(index)}
              className={cn(
                "p-4 rounded-lg border transition-colors cursor-pointer",
                selectedOption === index
                  ? "bg-primary/10 dark:bg-dark-primary border-primary dark:border-dark-primary"
                  : "bg-tool-card dark:bg-tool-card-dark hover:bg-accent dark:hover:bg-dark-accent"
              )}
            >
              <h3 className="font-semibold text-lg mb-2 dark:text-gray-100">{opt.optionTitle}</h3>
              <div className="prose prose-xs max-w-none dark:text-gray-100">
                <ReactMarkdown>{opt.optionDescription}</ReactMarkdown>
              </div>
            </div>
          ))}
          <Button
            onClick={handleSubmit}
            variant="default"
            className="w-full sm:w-auto dark:bg-button-dark"
            disabled={selectedOption === null}
          >
            <GPSIcon size={14} />
            Select plan
          </Button>
        </div>
      )}
    </div>
  );
}
