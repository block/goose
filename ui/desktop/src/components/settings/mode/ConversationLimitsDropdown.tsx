import { useState } from 'react';
import { ChevronDown } from 'lucide-react';
import { Input } from '../../ui/input';

interface ConversationLimitsDropdownProps {
  maxTurns: number;
  onMaxTurnsChange: (value: number) => void;
}

export const ConversationLimitsDropdown = ({
  maxTurns,
  onMaxTurnsChange,
}: ConversationLimitsDropdownProps) => {
  const [isExpanded, setIsExpanded] = useState(false);

  const toggleExpanded = () => {
    setIsExpanded(!isExpanded);
  };

  return (
    <div className="pt-4">
      <button
        onClick={toggleExpanded}
        className="w-full flex items-center justify-between py-2 px-2 hover:bg-background-secondary rounded-lg transition-all group"
      >
        <h3 className="text-text-primary">Conversation Limits</h3>

        <ChevronDown
          className={`w-4 h-4 text-text-secondary transition-transform duration-200 ease-in-out ${
            isExpanded ? 'rotate-180' : 'rotate-0'
          }`}
        />
      </button>

      <div
        className={`overflow-hidden transition-all duration-300 ease-in-out ${
          isExpanded ? 'max-h-96 opacity-100 mt-2' : 'max-h-0 opacity-0 mt-0'
        }`}
      >
        <div className="space-y-3 pb-2">
          <div className="flex items-center justify-between py-2 px-2 bg-background-secondary rounded-lg transform transition-all duration-200 ease-in-out">
            <div>
              <h4 className="text-text-primary text-sm">Max Turns</h4>
              <p className="text-xs text-text-secondary mt-[2px]">
                Maximum agent turns before Goose asks for user input
              </p>
            </div>
            <Input
              type="number"
              min="1"
              max="10000"
              value={maxTurns}
              onChange={(e) => onMaxTurnsChange(Number(e.target.value))}
              className="w-20"
            />
          </div>
        </div>
      </div>
    </div>
  );
};
