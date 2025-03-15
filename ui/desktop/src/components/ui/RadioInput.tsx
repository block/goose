import React from 'react';

export interface RadioInputProps {
  label: string;
  description: string | null;
  Accessory?: React.FC;
  value: string;
  isChecked: boolean;
  onClick: () => void;
}
export const RadioInput = ({
  label,
  description,
  Accessory,
  value,
  isChecked,
  onClick,
}: RadioInputProps) => {
  return (
    <div className="block cursor-pointer" onClick={onClick}>
      <div
        className="flex items-center justify-between p-2 text-textStandard hover:bg-bgSubtle transition-colors"
        onClick={onClick}
      >
        <div>
          <p className="text-sm ">{label}</p>
          <p className="text-xs text-textSubtle">{description}</p>
        </div>
        <div className="relative flex items-center gap-3">
          {Accessory ? <Accessory /> : null}
          <input
            type="radio"
            name="recentModels"
            value={value}
            checked={isChecked}
            onChange={onClick}
            className="peer sr-only"
          />
          <div
            className="h-4 w-4 rounded-full border border-gray-400 dark:border-gray-500
                          peer-checked:border-[6px] peer-checked:border-black dark:peer-checked:border-white
                          peer-checked:bg-white dark:peer-checked:bg-black
                          transition-all duration-200 ease-in-out"
          ></div>
        </div>
      </div>
    </div>
  );
};
