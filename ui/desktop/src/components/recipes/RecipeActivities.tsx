import GooseLogo from '../GooseLogo';
import MarkdownContent from '../MarkdownContent';
import { substituteParameters } from '../../utils/parameterSubstitution';

interface RecipeActivitiesProps {
  append: (text: string) => boolean | Promise<boolean>;
  activities: string[] | null;
  title?: string;
  parameterValues?: Record<string, string>;
  disabled?: boolean;
}

export default function RecipeActivities({
  append,
  activities,
  parameterValues = {},
  disabled = false,
}: RecipeActivitiesProps) {
  const pills = activities || [];

  // Find any pill that starts with "message:"
  const messagePillIndex = pills.findIndex((pill) => pill.toLowerCase().startsWith('message:'));

  // Extract the message pill and the remaining pills
  const messagePill = messagePillIndex >= 0 ? pills[messagePillIndex] : null;
  const remainingPills =
    messagePillIndex >= 0
      ? [...pills.slice(0, messagePillIndex), ...pills.slice(messagePillIndex + 1)]
      : pills;

  // If we have activities or instructions (recipe mode), show a simplified version without greeting
  if (activities && activities.length > 0) {
    return (
      <div className="flex flex-col px-6">
        {/* Animated goose icon */}
        <div className="flex justify-start mb-6">
          <GooseLogo size="default" hover={true} />
        </div>

        {messagePill && (
          <div className="mb-4 p-3 rounded-lg border animate-[fadein_500ms_ease-in_forwards]">
            <MarkdownContent
              content={substituteParameters(
                messagePill.replace(/^message:/i, '').trim(),
                parameterValues
              )}
              className="text-sm"
            />
          </div>
        )}

        <div className="flex flex-wrap gap-2 animate-[fadein_500ms_ease-in_forwards]">
          {remainingPills.map((content, index) => {
            const substitutedContent = substituteParameters(content, parameterValues);
            return (
              <button
                key={index}
                type="button"
                disabled={disabled}
                onClick={() => void append(substitutedContent)}
                title={substitutedContent.length > 60 ? substitutedContent : undefined}
                className="bg-background-primary text-text-primary rounded-xl border shadow-sm cursor-pointer px-3 py-1.5 text-sm text-left hover:bg-background-secondary transition-colors disabled:cursor-not-allowed disabled:opacity-50 disabled:hover:bg-background-primary"
              >
                {substitutedContent.length > 60
                  ? substitutedContent.slice(0, 60) + '...'
                  : substitutedContent}
              </button>
            );
          })}
        </div>
      </div>
    );
  }

  return null;
}
