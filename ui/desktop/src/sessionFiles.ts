import { SessionDetails } from './sessions';

/**
 * File format for exported sessions
 */
export interface SessionExportFormat {
  version: string;
  timestamp: number;
  session: SessionDetails;
}

/**
 * Exports a session to a JSON file for download
 * @param session The session to export
 * @returns A Blob containing the session data
 */
export function exportSessionToFile(session: SessionDetails): globalThis.Blob {
  const exportData: SessionExportFormat = {
    version: '1.0',
    timestamp: Date.now(),
    session,
  };

  const jsonString = JSON.stringify(exportData, null, 2);
  return new globalThis.Blob([jsonString], { type: 'application/json' });
}

/**
 * Triggers a download of the session as a JSON file
 * @param session The session to download
 */
export function downloadSession(session: SessionDetails): void {
  const blob = exportSessionToFile(session);
  const filename = `goose-session-${session.session_id}.egg`;

  // Create a download link and trigger the download
  const url = URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.href = url;
  link.download = filename;
  document.body.appendChild(link);
  link.click();

  // Clean up
  setTimeout(() => {
    document.body.removeChild(link);
    URL.revokeObjectURL(url);
  }, 100);
}

/**
 * Imports a session from a file
 * @param file The file to import
 * @returns Promise with the imported session details
 */
export async function importSessionFromFile(file: globalThis.File): Promise<SessionDetails> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();

    reader.onload = (event) => {
      try {
        const content = event.target?.result as string;
        const data = JSON.parse(content) as SessionExportFormat;

        // Validate the imported data
        if (!data.version || !data.session || !data.session.session_id) {
          throw new Error('Invalid session file format');
        }

        resolve(data.session);
      } catch (error) {
        reject(error instanceof Error ? error : new Error('Failed to parse session file'));
      }
    };

    reader.onerror = () => {
      reject(new Error('Failed to read session file'));
    };

    reader.readAsText(file);
  });
}
