import { useState, useEffect } from 'react';
import { Button } from '../ui/button';

export default function RecipeActivityEditor({
  activities = [],
  setActivities,
  onBlur,
}: {
  activities?: string[];
  setActivities: (prev: string[]) => void;
  onBlur?: () => void;
}) {
  const [newActivity, setNewActivity] = useState('');
  const [messageContent, setMessageContent] = useState('');

  // Extract message content from activities on component mount and when activities change
  useEffect(() => {
    const messageActivity = activities.find((activity) =>
      activity.toLowerCase().startsWith('message:')
    );
    if (messageActivity) {
      setMessageContent(messageActivity.replace(/^message:/i, ''));
    }
  }, [activities]);

  // Get activities that are not messages
  const nonMessageActivities = activities.filter(
    (activity) => !activity.toLowerCase().startsWith('message:')
  );

  const handleAddActivity = () => {
    if (newActivity.trim()) {
      setActivities([...activities, newActivity.trim()]);
      setNewActivity('');
      // Trigger parameter extraction after adding activity
      if (onBlur) {
        onBlur();
      }
    }
  };

  const handleRemoveActivity = (activity: string) => {
    setActivities(activities.filter((a) => a !== activity));
  };

  const handleMessageChange = (value: string) => {
    setMessageContent(value);

    // Update activities array - remove existing message and add new one if not empty
    const otherActivities = activities.filter(
      (activity) => !activity.toLowerCase().startsWith('message:')
    );

    if (value.length > 0) {
      setActivities([`message:${value}`, ...otherActivities]);
    } else {
      setActivities(otherActivities);
    }
  };

  return (
    <div>
      <label htmlFor="activities" className="block text-md text-textProminent mb-2 font-bold">
        Activities
      </label>
      <p className="text-sm text-textSubtle space-y-2 pb-4">
        The top-line prompts and activity buttons that will display in the recipe chat window.
      </p>

      {/* Message Field */}
      <div>
        <label htmlFor="message" className="block text-sm font-medium text-textStandard mb-2">
          Message
        </label>
        <p className="text-xs text-textSubtle mb-2">
          A formatted message that will appear at the top of the recipe. Supports markdown
          formatting.
        </p>
        <textarea
          id="message"
          value={messageContent}
          onChange={(e) => handleMessageChange(e.target.value)}
          onBlur={onBlur}
          className="w-full px-4 py-3 border rounded-lg bg-background-default text-textStandard placeholder-textPlaceholder focus:outline-none focus:ring-2 focus:ring-borderProminent resize-vertical"
          placeholder="Enter a user facing introduction message for your recipe (supports **bold**, *italic*, `code`, etc.)"
          rows={3}
          autoCorrect="off"
          autoCapitalize="off"
          spellCheck="false"
        />
      </div>

      {/* Regular Activities */}
      <div className="space-y-4">
        <div>
          <label className="block text-sm font-medium text-textStandard mb-2">
            Activity Buttons
          </label>
          <p className="text-xs text-textSubtle mb-3">
            Clickable buttons that will appear below the message to help users interact with your
            recipe.
          </p>
        </div>

        {nonMessageActivities.length > 0 && (
          <div className="flex flex-wrap gap-3">
            {nonMessageActivities.map((activity, index) => (
              <div
                key={index}
                className="inline-flex items-center bg-background-default border-2 border-borderSubtle rounded-full px-4 py-2 text-sm text-textStandard"
                title={activity.length > 100 ? activity : undefined}
              >
                <span>{activity.length > 100 ? activity.slice(0, 100) + '...' : activity}</span>
                <Button
                  type="button"
                  onClick={() => handleRemoveActivity(activity)}
                  variant="ghost"
                  size="sm"
                  className="ml-2 text-textStandard hover:text-textSubtle transition-colors p-0 h-auto"
                >
                  ×
                </Button>
              </div>
            ))}
          </div>
        )}
        <div className="flex gap-2 mt-3">
          <input
            type="text"
            value={newActivity}
            onChange={(e) => setNewActivity(e.target.value)}
            onKeyPress={(e) => e.key === 'Enter' && handleAddActivity()}
            onBlur={onBlur}
            className="flex-1 px-3 py-2 border border-border-subtle rounded-lg bg-background-default text-text-standard focus:outline-none focus:ring-2 focus:ring-blue-500 text-sm"
            placeholder="Add new activity..."
          />
          <button
            type="button"
            onClick={handleAddActivity}
            disabled={!newActivity.trim()}
            className="px-4 py-2 bg-blue-500 text-white rounded-lg text-sm hover:bg-blue-600 transition-colors disabled:bg-gray-400 disabled:cursor-not-allowed"
          >
            Add activity
          </button>
        </div>
      </div>
    </div>
  );
}
