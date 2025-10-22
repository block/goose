import { ScheduledJob } from '../schedule';
import { RecipeManifestResponse } from '../api';
import { getStorageDirectory } from '../recipe/recipe_management';

/**
 * Match recipes with their schedules based on file paths
 */
export function matchRecipesWithSchedules(
  recipes: RecipeManifestResponse[],
  schedules: ScheduledJob[]
): Map<string, ScheduledJob> {
  const recipeScheduleMap = new Map<string, ScheduledJob>();
  
  // Get the storage directory path
  const storageDir = getStorageDirectory(false); // false for user recipes
  
  for (const recipe of recipes) {
    // Construct the expected file path for this recipe
    const recipeFilePath = `${storageDir}/${recipe.id}.yaml`;
    
    // Find a schedule that matches this recipe's file path
    const matchingSchedule = schedules.find(schedule => {
      // The schedule source should match the recipe file path
      return schedule.source === recipeFilePath || 
             schedule.source.endsWith(`/${recipe.id}.yaml`);
    });
    
    if (matchingSchedule) {
      recipeScheduleMap.set(recipe.id, matchingSchedule);
    }
  }
  
  return recipeScheduleMap;
}

/**
 * Calculate the next run time for a schedule
 * Returns a human-readable string or null if schedule is paused
 */
export function getNextRunTime(schedule: ScheduledJob): string | null {
  if (schedule.paused) {
    return null;
  }
  
  if (schedule.currently_running) {
    return 'Running now';
  }
  
  // For now, we'll use a simple approach
  // In the future, we could use a cron parser to calculate exact next run time
  // For now, just show the cron expression in a readable way
  return parseCronToNextRun(schedule.cron);
}

/**
 * Simple cron parser to estimate next run time
 * This is a basic implementation - could be enhanced with a proper cron library
 */
function parseCronToNextRun(cron: string): string {
  // This is a simplified version
  // In production, you'd want to use a library like 'cron-parser' or 'cronstrue'
  const parts = cron.split(' ');
  
  if (parts.length < 5) {
    return 'Invalid cron';
  }
  
  const [minute, hour, dayOfMonth, month, dayOfWeek] = parts;
  
  // Daily pattern: specific time every day
  if (dayOfMonth === '*' && month === '*' && dayOfWeek === '*') {
    if (minute !== '*' && hour !== '*') {
      return `Daily at ${hour.padStart(2, '0')}:${minute.padStart(2, '0')}`;
    }
  }
  
  // Interval patterns
  if (minute.startsWith('*/')) {
    const interval = minute.substring(2);
    return `Every ${interval} minutes`;
  }
  
  if (hour.startsWith('*/')) {
    const interval = hour.substring(2);
    return `Every ${interval} hours`;
  }
  
  // Weekly pattern
  if (dayOfWeek !== '*' && hour !== '*' && minute !== '*') {
    const days = dayOfWeek.split(',').map(d => {
      const dayNames = ['Sunday', 'Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday'];
      return dayNames[parseInt(d)] || d;
    }).join(', ');
    return `${days} at ${hour.padStart(2, '0')}:${minute.padStart(2, '0')}`;
  }
  
  // Monthly pattern
  if (dayOfMonth !== '*' && hour !== '*' && minute !== '*') {
    return `Monthly on day ${dayOfMonth} at ${hour.padStart(2, '0')}:${minute.padStart(2, '0')}`;
  }
  
  // Fallback to showing the cron expression
  return `Cron: ${cron}`;
}

/**
 * Format a date string to relative time (e.g., "2 hours ago")
 */
export function formatRelativeTime(dateString: string | null | undefined): string | null {
  if (!dateString) {
    return null;
  }
  
  try {
    const date = new Date(dateString);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMins = Math.floor(diffMs / 60000);
    const diffHours = Math.floor(diffMins / 60);
    const diffDays = Math.floor(diffHours / 24);
    
    if (diffMins < 1) {
      return 'Just now';
    } else if (diffMins < 60) {
      return `${diffMins} minute${diffMins !== 1 ? 's' : ''} ago`;
    } else if (diffHours < 24) {
      return `${diffHours} hour${diffHours !== 1 ? 's' : ''} ago`;
    } else if (diffDays < 7) {
      return `${diffDays} day${diffDays !== 1 ? 's' : ''} ago`;
    } else {
      return date.toLocaleDateString();
    }
  } catch (error) {
    console.error('Error formatting relative time:', error);
    return null;
  }
}
