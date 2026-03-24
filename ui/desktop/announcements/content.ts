// Map of announcement file names to their content
export const announcementContents: Record<string, string> = {};

// Helper function to get announcement content by filename
export function getAnnouncementContent(filename: string): string | null {
  return announcementContents[filename] || null;
}
