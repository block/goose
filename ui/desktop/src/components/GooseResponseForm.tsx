import React, { useState } from 'react';
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

  let isReady = false;
  let isQuestion = false;
  let isOptions = false;
  let options = [];

  if (metadata) {
    isReady = metadata[0] === "READY";
    isQuestion = metadata[0] === "QUESTION";
    isOptions = metadata[0] === "OPTIONS";

    if (isOptions && metadata[1]) {
      try {
        let optionsData = metadata[1];
        if (optionsData.startsWith('```json')) {
          optionsData = optionsData.replace(/```json/g, '').replace(/```/g, '');
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

  if (isQuestion || isOptions) {    
    window.electron.showNotification({
      title: 'Goose has a question for you',
      body: `please check with goose to approve the plan of action`,
    });
  }

  return (
    <div className="space-y-4">
      {(!isOptions || options.length === 0) && (
        <div className="prose max-w-none">
          <ReactMarkdown>{message}</ReactMarkdown>
        </div>
      )}
      {isQuestion && (
        <div className="flex items-center gap-4 p-4 rounded-lg bg-tool-card border">
          <Button
            onClick={handleAccept}
            variant="default"
            className="w-full sm:w-auto"
          >
            <GPSIcon size={14} />
            Take flight with this plan
          </Button>
          <Button
            onClick={handleCancel}
            variant="destructive"
            className="w-full sm:w-auto"
          >
            <GPSIcon size={14} />
            Cancel
          </Button>
        </div>
      )}
      {isOptions && options.length > 0 && (
        <div className="space-y-4">
          {options.map((opt, index) => (
            <div
              key={index}
              onClick={() => handleOptionClick(index)}
              className={cn(
                "p-4 rounded-lg border transition-colors cursor-pointer",
                selectedOption === index
                  ? "bg-primary/10 border-primary"
                  : "bg-tool-card hover:bg-accent"
              )}
            >
              <h3 className="font-semibold text-lg mb-2">{opt.optionTitle}</h3>
              <div className="prose max-w-none">
                <ReactMarkdown>{opt.optionDescription}</ReactMarkdown>
              </div>
            </div>
          ))}
          <Button
            onClick={handleSubmit}
            variant="default"
            className="w-full sm:w-auto"
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