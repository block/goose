import { SessionDetails, SharedSessionDetails } from './sessions';

export type View =
  | 'welcome'
  | 'chat'
  | 'settings'
  | 'moreModels'
  | 'configureProviders'
  | 'configPage'
  | 'ConfigureProviders'
  | 'settingsV2'
  | 'sessions'
  | 'sharedSession';

export interface ViewOptionsBase {
  resumedSession?: SessionDetails;
  shareToken?: string;
  baseUrl?: string;
  sessionDetails?: SharedSessionDetails;
  error?: string;
}

export interface ViewConfig {
  view: View;
  viewOptions?: ViewOptionsBase;
}
