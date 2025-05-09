import { useEffect, useState } from 'react';

export interface ResponseStyle {
  key: string;
  label: string;
  description: string;
}

export const all_response_styles: ResponseStyle[] = [
  {
    key: 'detailed',
    label: 'Detailed',
    description: 'Tool calls are by default shown open to expose details',
  },
  {
    key: 'concise',
    label: 'Concise',
    description: 'Tool calls are by default closed and only show the tool used',
  },
];

interface ResponseStyleSelectionItemProps {
  currentStyle: string;
  style: ResponseStyle;
  showDescription: boolean;
  handleStyleChange: (newStyle: string) => void;
}

export function ResponseStyleSelectionItem({
  currentStyle,
  style,
  showDescription,
  handleStyleChange,
}: ResponseStyleSelectionItemProps) {
  const [checked, setChecked] = useState(currentStyle === style.key);

  useEffect(() => {
    setChecked(currentStyle === style.key);
  }, [currentStyle, style.key]);

  return (
    <div className="group hover:cursor-pointer">
      <div
        className="flex items-center justify-between text-textStandard py-2 px-4 hover:bg-bgSubtle"
        onClick={() => handleStyleChange(style.key)}
      >
        <div className="flex">
          <div>
            <h3 className="text-textStandard">{style.label}</h3>
            {showDescription && (
              <p className="text-xs text-textSubtle max-w-md mt-[2px]">{style.description}</p>
            )}
          </div>
        </div>

        <div className="relative flex items-center gap-2">
          <input
            type="radio"
            name="responseStyles"
            value={style.key}
            checked={checked}
            onChange={() => handleStyleChange(style.key)}
            className="peer sr-only"
          />
          <div
            className="h-4 w-4 rounded-full border border-borderStandard 
                  peer-checked:border-[6px] peer-checked:border-black dark:peer-checked:border-white
                  peer-checked:bg-white dark:peer-checked:bg-black
                  transition-all duration-200 ease-in-out group-hover:border-borderProminent"
          ></div>
        </div>
      </div>
    </div>
  );
}
