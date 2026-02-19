import type { Session } from '../api';

export interface DateGroup {
  label: string;
  sessions: Session[];
  date: Date;
}

export function groupSessionsByDate(sessions: Session[]): DateGroup[] {
  const today = new Date();
  today.setHours(0, 0, 0, 0);

  const yesterday = new Date(today);
  yesterday.setDate(yesterday.getDate() - 1);

  const groups: { [key: string]: DateGroup } = {};

  sessions.forEach((session) => {
    const sessionDate = new Date(session.updated_at);
    const sessionDateStart = new Date(sessionDate);
    sessionDateStart.setHours(0, 0, 0, 0);

    let label: string;
    let groupKey: string;

    if (sessionDateStart.getTime() === today.getTime()) {
      label = 'Today';
      groupKey = 'today';
    } else if (sessionDateStart.getTime() === yesterday.getTime()) {
      label = 'Yesterday';
      groupKey = 'yesterday';
    } else {
      // Format as "Monday, January 1" or "January 1" if it's not this year
      const currentYear = today.getFullYear();
      const sessionYear = sessionDateStart.getFullYear();

      if (sessionYear === currentYear) {
        label = sessionDateStart.toLocaleDateString('en-US', {
          weekday: 'long',
          month: 'long',
          day: 'numeric',
        });
      } else {
        label = sessionDateStart.toLocaleDateString('en-US', {
          month: 'long',
          day: 'numeric',
          year: 'numeric',
        });
      }
      groupKey = sessionDateStart.toISOString().split('T')[0];
    }

    if (!groups[groupKey]) {
      groups[groupKey] = {
        label,
        sessions: [],
        date: sessionDateStart,
      };
    }

    groups[groupKey].sessions.push(session);
  });

  // Convert to array and sort by date (newest first)
  return Object.values(groups).sort((a, b) => b.date.getTime() - a.date.getTime());
}

export interface ProjectGroup {
  project: string;
  sessionCount: number;
  dateGroups: DateGroup[];
}

function getProjectName(workingDir: string): string {
  if (!workingDir) return 'General';
  const parts = workingDir.replace(/\/+$/, '').split('/');
  return parts[parts.length - 1] || 'General';
}

export function groupSessionsByProjectThenDate(sessions: Session[]): ProjectGroup[] {
  const projectMap: Record<string, Session[]> = {};

  sessions.forEach((session) => {
    const project = getProjectName(session.working_dir);
    if (!projectMap[project]) {
      projectMap[project] = [];
    }
    projectMap[project].push(session);
  });

  const projectGroups: ProjectGroup[] = Object.entries(projectMap).map(
    ([project, projectSessions]) => ({
      project,
      sessionCount: projectSessions.length,
      dateGroups: groupSessionsByDate(projectSessions),
    })
  );

  // Sort: 'General' last, others by most recent session
  return projectGroups.sort((a, b) => {
    if (a.project === 'General') return 1;
    if (b.project === 'General') return -1;
    const aLatest = a.dateGroups[0]?.date.getTime() ?? 0;
    const bLatest = b.dateGroups[0]?.date.getTime() ?? 0;
    return bLatest - aLatest;
  });
}
