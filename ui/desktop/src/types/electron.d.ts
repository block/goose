declare namespace ElectronTypes {
  interface Event {
    preventDefault: () => void;
    sender: unknown;
  }

  interface IpcRendererEvent extends Event {
    senderId: number;
  }

  interface MouseUpEvent extends Event {
    button: number;
  }
}
