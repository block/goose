// Store for pending new chat configuration
// Holds state that will be applied when creating a new session
// This allows changing settings from Hub before any session exists

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
