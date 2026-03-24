export interface ElectronEvent {
  preventDefault: () => void;
  sender: unknown;
}

export interface IpcRendererEvent extends ElectronEvent {
  senderId: number;
}

// Mouse event
export interface MouseUpEvent extends ElectronEvent {
  button: number;
}
