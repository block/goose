import { useEffect, useState } from 'react';
import { getReasoningEffort, setReasoningEffort } from '../../../api';

const REASONING_LEVELS = [
  {
    key: 'low' as const,
    label: 'Low',
    description: 'Faster responses, lighter reasoning',
  },
  {
    key: 'medium' as const,
    label: 'Medium',
    description: 'Balanced speed and depth (default)',
  },
  {
    key: 'high' as const,
    label: 'High',
    description: 'Deeper reasoning, higher latency',
  },
];

export function ReasoningEffortSection() {
  const [currentLevel, setCurrentLevel] = useState<string>('medium');
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    async function loadLevel() {
      try {
        const response = await getReasoningEffort();
        if (response.data?.level) {
          setCurrentLevel(response.data.level);
        }
      } catch (error) {
        console.error('Error loading reasoning effort:', error);
      } finally {
        setIsLoading(false);
      }
    }
    loadLevel();
  }, []);

  const handleLevelChange = async (newLevel: string) => {
    const previousLevel = currentLevel;
    setCurrentLevel(newLevel);
    try {
      await setReasoningEffort({ body: { level: newLevel } });
    } catch (error) {
      console.error('Error setting reasoning effort:', error);
      setCurrentLevel(previousLevel);
    }
  };

  if (isLoading) {
    return null;
  }

  return (
    <div className="space-y-1">
      {REASONING_LEVELS.map((level) => {
        const checked = currentLevel === level.key;
        return (
          <div key={level.key} className="group hover:cursor-pointer text-sm">
            <div
              className={`flex items-center justify-between text-text-primary py-2 px-2 ${
                checked
                  ? 'bg-background-secondary'
                  : 'bg-background-primary hover:bg-background-secondary'
              } rounded-lg transition-all`}
              onClick={() => handleLevelChange(level.key)}
            >
              <div>
                <h3 className="text-text-primary">{level.label}</h3>
                <p className="text-xs text-text-secondary mt-[2px]">{level.description}</p>
              </div>

              <div className="relative flex items-center gap-2">
                <input
                  type="radio"
                  name="reasoningEffort"
                  value={level.key}
                  checked={checked}
                  onChange={() => handleLevelChange(level.key)}
                  className="peer sr-only"
                />
                <div
                  className="h-4 w-4 rounded-full border border-border-primary 
                    peer-checked:border-[6px] peer-checked:border-black dark:peer-checked:border-white
                    peer-checked:bg-white dark:peer-checked:bg-black
                    transition-all duration-200 ease-in-out group-hover:border-border-primary"
                ></div>
              </div>
            </div>
          </div>
        );
      })}
    </div>
  );
}
