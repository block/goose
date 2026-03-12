import type { WhiteLabelConfig } from './types';

export const DEFAULT_WHITELABEL_CONFIG: WhiteLabelConfig = {
  branding: {
    appName: 'Goose',
    greetings: [
      'What would you like to work on?',
      'Ready to build something amazing?',
      'What shall we create today?',
      "What's on your mind?",
      'What project needs attention?',
      'What would you like to tackle?',
      'What would you like to explore?',
      'What needs to be done?',
      "What's the plan for today?",
      'Ready to create something great?',
    ],
  },
  features: {
    updatesEnabled: true,
    costTrackingEnabled: true,
    announcementsEnabled: false,
    configurationEnabled: true,
    telemetryUiEnabled: true,
    navigation: ['home', 'chat', 'recipes', 'apps', 'scheduler', 'extensions', 'settings'],
    settingsTabs: ['models', 'local-inference', 'chat', 'sharing', 'prompts', 'keyboard', 'app'],
    showModelSelector: true,
  },
  defaults: {},
  window: {
    width: 940,
    height: 800,
    minWidth: 450,
  },
};
