export interface CustomSubmitEvent extends CustomEvent {
  detail: {
    value: string;
    image?: {
      preview: string;
      compressed: string;
      path?: string;
    };
  };
}