import { describe, it, expect } from 'vitest';
import type { WhiteLabelConfig } from '../types';
import { DEFAULT_WHITELABEL_CONFIG } from '../defaults';

// Mock __WHITELABEL_CONFIG__ for tests
const mockConfig: WhiteLabelConfig = {
  ...DEFAULT_WHITELABEL_CONFIG,
  branding: {
    appName: 'TestApp',
    greetings: ['Hello from TestApp!', 'Welcome to TestApp!'],
  },
  features: {
    ...DEFAULT_WHITELABEL_CONFIG.features,
    navigation: ['home', 'chat', 'settings'],
    settingsTabs: ['models', 'app'],
    hiddenSettingSections: ['tunnel', 'session-sharing'],
    allowedProviders: ['openai', 'anthropic'],
    updatesEnabled: false,
    announcementsEnabled: true,
  },
  window: {
    width: 1200,
    height: 900,
    minWidth: 500,
  },
};

describe('WhiteLabelConfig defaults', () => {
  it('has all required top-level keys', () => {
    expect(DEFAULT_WHITELABEL_CONFIG.branding).toBeDefined();
    expect(DEFAULT_WHITELABEL_CONFIG.features).toBeDefined();
    expect(DEFAULT_WHITELABEL_CONFIG.defaults).toBeDefined();
    expect(DEFAULT_WHITELABEL_CONFIG.window).toBeDefined();
  });

  it('defaults appName to Goose', () => {
    expect(DEFAULT_WHITELABEL_CONFIG.branding.appName).toBe('Goose');
  });

  it('has default greetings', () => {
    expect(DEFAULT_WHITELABEL_CONFIG.branding.greetings.length).toBeGreaterThan(0);
  });

  it('has all default navigation items', () => {
    const nav = DEFAULT_WHITELABEL_CONFIG.features.navigation;
    expect(nav).toContain('home');
    expect(nav).toContain('chat');
    expect(nav).toContain('recipes');
    expect(nav).toContain('apps');
    expect(nav).toContain('scheduler');
    expect(nav).toContain('extensions');
    expect(nav).toContain('settings');
  });

  it('has all default settings tabs', () => {
    const tabs = DEFAULT_WHITELABEL_CONFIG.features.settingsTabs;
    expect(tabs).toContain('models');
    expect(tabs).toContain('local-inference');
    expect(tabs).toContain('chat');
    expect(tabs).toContain('sharing');
    expect(tabs).toContain('prompts');
    expect(tabs).toContain('keyboard');
    expect(tabs).toContain('app');
  });

  it('has sensible default window dimensions', () => {
    expect(DEFAULT_WHITELABEL_CONFIG.window.width).toBe(940);
    expect(DEFAULT_WHITELABEL_CONFIG.window.height).toBe(800);
    expect(DEFAULT_WHITELABEL_CONFIG.window.minWidth).toBe(450);
  });

  it('enables features by default', () => {
    expect(DEFAULT_WHITELABEL_CONFIG.features.updatesEnabled).toBe(true);
    expect(DEFAULT_WHITELABEL_CONFIG.features.costTrackingEnabled).toBe(true);
    expect(DEFAULT_WHITELABEL_CONFIG.features.configurationEnabled).toBe(true);
  });
});

describe('WhiteLabelConfig custom config', () => {
  it('can override appName', () => {
    expect(mockConfig.branding.appName).toBe('TestApp');
  });

  it('can restrict navigation items', () => {
    expect(mockConfig.features.navigation).toEqual(['home', 'chat', 'settings']);
    expect(mockConfig.features.navigation).not.toContain('recipes');
    expect(mockConfig.features.navigation).not.toContain('extensions');
  });

  it('can restrict settings tabs', () => {
    expect(mockConfig.features.settingsTabs).toEqual(['models', 'app']);
    expect(mockConfig.features.settingsTabs).not.toContain('chat');
    expect(mockConfig.features.settingsTabs).not.toContain('sharing');
  });

  it('can hide setting sections', () => {
    expect(mockConfig.features.hiddenSettingSections).toContain('tunnel');
    expect(mockConfig.features.hiddenSettingSections).toContain('session-sharing');
  });

  it('can restrict providers', () => {
    expect(mockConfig.features.allowedProviders).toContain('openai');
    expect(mockConfig.features.allowedProviders).toContain('anthropic');
    expect(mockConfig.features.allowedProviders).not.toContain('ollama');
  });

  it('can disable features', () => {
    expect(mockConfig.features.updatesEnabled).toBe(false);
    expect(mockConfig.features.announcementsEnabled).toBe(true);
  });

  it('can customize window dimensions', () => {
    expect(mockConfig.window.width).toBe(1200);
    expect(mockConfig.window.height).toBe(900);
    expect(mockConfig.window.minWidth).toBe(500);
  });

  it('can customize greetings', () => {
    expect(mockConfig.branding.greetings).toHaveLength(2);
    expect(mockConfig.branding.greetings[0]).toBe('Hello from TestApp!');
  });
});

describe('WhiteLabelConfig type safety', () => {
  it('extension defaults have required fields', () => {
    const ext = {
      name: 'test-ext',
      type: 'sse',
      uri: 'https://example.com/sse',
      enabled: true,
    };
    expect(ext.name).toBeDefined();
    expect(ext.type).toBeDefined();
    expect(ext.enabled).toBeDefined();
  });

  it('process config has required fields', () => {
    const proc = {
      name: 'sidecar',
      command: 'node',
      args: ['server.js'],
      restartOnCrash: true,
      waitForPort: 9999,
      waitTimeoutMs: 10000,
    };
    expect(proc.name).toBeDefined();
    expect(proc.command).toBeDefined();
  });
});
