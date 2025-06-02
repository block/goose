import { useState } from 'react';

export default function RecipeActivityEditor({
  activities,
  setActivities,
}: {
  activities: string[];
  setActivities: (prev: string[]) => void;
}) {
  const [newActivity, setNewActivity] = useState('');
  const handleAddActivity = () => {
    if (newActivity.trim()) {
      setActivities([...activities, newActivity.trim()]);
      setNewActivity('');
    }
  };

  const handleRemoveActivity = (activity: string) => {
    setActivities(activities.filter((a) => a !== activity));
  };
  return (
    <div>
      <label htmlFor="activities" className="block text-md text-textProminent mb-2 font-bold">
        Activities
      </label>
      <p className="text-textSubtle space-y-2 pb-2">
        The top-line prompts and activities that will display within your goose home page.
      </p>
      <div className="space-y-4">
        <div className="flex flex-wrap gap-3">
          {activities.map((activity, index) => (
            <div
              key={index}
              className="inline-flex items-center bg-bgApp border-2 border-borderSubtle rounded-full px-4 py-2 text-sm text-textStandard"
              title={activity.length > 100 ? activity : undefined}
            >
              <span>{activity.length > 100 ? activity.slice(0, 100) + '...' : activity}</span>
              <button
                onClick={() => handleRemoveActivity(activity)}
                className="ml-2 text-textStandard hover:text-textSubtle transition-colors"
              >
                ×
              </button>
            </div>
          ))}
        </div>
        <div className="flex gap-3 mt-6">
          <input
            type="text"
            value={newActivity}
            onChange={(e) => setNewActivity(e.target.value)}
            onKeyPress={(e) => e.key === 'Enter' && handleAddActivity()}
            className="flex-1 px-4 py-3 border rounded-lg bg-bgApp text-textStandard placeholder-textPlaceholder focus:outline-none focus:ring-2 focus:ring-borderProminent"
            placeholder="Add new activity..."
          />
          <button
            onClick={handleAddActivity}
            className="px-5 py-1.5 text-sm bg-bgAppInverse text-textProminentInverse rounded-xl hover:bg-bgStandardInverse transition-colors"
          >
            Add activity
          </button>
        </div>
      </div>
    </div>
  );
}
