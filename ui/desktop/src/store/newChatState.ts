// Store for new chat configuration
// Acts as a cache that can be updated from UI or synced from session
// Resets on page refresh - defaults to window.appConfig.get('GOOSE_WORKING_DIR')

interface NewChatState {
  workingDir: string | null;
  // Future additions:
  // extensions?: string[];
  // provider?: string;
  // model?: string;
}

const state: NewChatState = {
  workingDir: null,
};

export function setWorkingDir(dir: string): void {
  state.workingDir = dir;
}

export function getWorkingDir(): string {
  return state.workingDir ?? (window.appConfig.get('GOOSE_WORKING_DIR') as string);
}

export function clearWorkingDir(): void {
  state.workingDir = null;
}

// Generic getters/setters for future extensibility
export function getNewChatState(): Readonly<NewChatState> {
  return { ...state };
}

export function resetNewChatState(): void {
  state.workingDir = null;
  // Reset future fields here
}
