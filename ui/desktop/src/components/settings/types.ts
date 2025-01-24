import { FullExtensionConfig } from '../../extensions';

export interface Model {
  id: string;
  name: string;
  description: string;
  enabled: boolean;
}

export interface Settings {
  models: Model[];
  extensions: FullExtensionConfig[];
}
