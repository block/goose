/**
 * Utility to track MCP processes and register them with the main process
 */

/**
 * Tracks MCP processes by name pattern, finding their PIDs and registering them
 * with the electron main process for automatic cleanup when UI exits.
 *
 * @param processNamePattern The pattern to look for in MCP process names
 */
export async function trackMcpProcesses(processNamePattern: string): Promise<void> {
  if (process.platform === 'win32') {
    console.log('Process tracking not implemented for Windows yet');
    return;
  }

  // Wait a moment to give the process time to start
  await new Promise((resolve) => setTimeout(resolve, 1000));

  try {
    // Use ps command to find processes matching our pattern
    const command = `ps -eo pid,command | grep "${processNamePattern}" | grep -v grep`;

    // Execute the command through the browser's fetch API to avoid CORS issues
    const response = await fetch(`http://localhost:${window.appConfig.get('GOOSE_PORT')}/command`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-Secret-Key': window.appConfig.get('secretKey') as string,
      },
      body: JSON.stringify({ command }),
    });

    if (!response.ok) {
      console.error('Failed to execute ps command:', response.statusText);
      return;
    }

    const result = await response.json();
    if (result.error) {
      console.error('Error executing ps command:', result.stderr || result.error);
      return;
    }

    // Parse the output to extract PIDs
    const lines = result.stdout.split('\n').filter(Boolean);
    for (const line of lines) {
      const parts = line.trim().split(/\s+/);
      if (parts.length < 2) continue;

      const pid = parseInt(parts[0], 10);
      if (isNaN(pid)) continue;

      console.log(`Found MCP process: PID=${pid}, Command=${parts.slice(1).join(' ')}`);

      // Register this PID with the main process
      const registered = await window.electron.registerMcpPid(pid);
      if (registered) {
        console.log(`Successfully registered MCP process PID ${pid} for tracking`);
      }
    }
  } catch (error) {
    console.error('Error tracking MCP processes:', error);
  }
}
