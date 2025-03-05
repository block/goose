import * as RadioGroup from '@radix-ui/react-radio-group';
import React from 'react';

const ModeSelection = ({ value, onChange }) => {
  const modes = [
    {
      value: 'auto',
      label: 'Completely autonomous',
      description: 'Full file modification capabilities, edit, create, and delete files freely.',
    },
    {
      value: 'approve',
      label: 'Approval needed',
      description: 'Editing, creating, and deleting files will require human approval.',
    },
    {
      value: 'chat',
      label: 'Chat only',
      description: 'Engage with the selected provider without using tools or extensions.',
    },
  ];

  return (
    <div>
      <h4 className="font-medium mb-4 text-textStandard">Mode Selection</h4>

      <RadioGroup.Root className="flex flex-col space-y-2" value={value} onValueChange={onChange}>
        {modes.map((mode) => (
          <RadioGroup.Item
            key={mode.value}
            value={mode.value}
            className="flex items-center justify-between p-2 hover:bg-gray-100 dark:hover:bg-gray-700 rounded transition-all cursor-pointer"
          >
            <div className="flex flex-col text-left">
              <h3 className="text-sm font-semibold text-textStandard dark:text-gray-200">
                {mode.label}
              </h3>
              <p className="text-xs text-textSubtle dark:text-gray-400 mt-[2px]">
                {mode.description}
              </p>
            </div>
            <div className="flex-shrink-0">
              <div className="w-4 h-4 flex items-center justify-center rounded-full border border-gray-500 dark:border-gray-400">
                {value === mode.value && (
                  <div className="w-2 h-2 bg-black dark:bg-white rounded-full" />
                )}
              </div>
            </div>
          </RadioGroup.Item>
        ))}
      </RadioGroup.Root>
    </div>
  );
};

export default ModeSelection;
