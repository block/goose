import React, { createContext, useContext, useMemo } from 'react';
import type {
  WhiteLabelConfig,
  WhiteLabelBranding,
  WhiteLabelFeatures,
  WhiteLabelDefaults,
  WhiteLabelWindow,
} from './types';
import { DEFAULT_WHITELABEL_CONFIG } from './defaults';

interface WhiteLabelContextType {
  config: WhiteLabelConfig;
  branding: WhiteLabelBranding;
  features: WhiteLabelFeatures;
  defaults: WhiteLabelDefaults;
  window: WhiteLabelWindow;
  isFeatureEnabled: (feature: string) => boolean;
  isNavItemEnabled: (itemId: string) => boolean;
  getNavLabel: (itemId: string, defaultLabel: string) => string;
  isSettingsTabEnabled: (tabId: string) => boolean;
  isSectionHidden: (sectionId: string) => boolean;
  isProviderAllowed: (providerId: string) => boolean;
  getRandomGreeting: () => string;
}

const WhiteLabelContext = createContext<WhiteLabelContextType | undefined>(undefined);

function getConfig(): WhiteLabelConfig {
  try {
    return __WHITELABEL_CONFIG__;
  } catch {
    return DEFAULT_WHITELABEL_CONFIG;
  }
}

export const WhiteLabelProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const config = useMemo(() => getConfig(), []);

  const value = useMemo<WhiteLabelContextType>(() => {
    const { branding, features, defaults, window: windowConfig } = config;

    const navSet = new Set(features.navigation);
    const tabSet = new Set(features.settingsTabs);
    const hiddenSections = new Set(features.hiddenSettingSections || []);
    const allowedProviders = features.allowedProviders
      ? new Set(features.allowedProviders.map((p) => p.toLowerCase()))
      : null;

    return {
      config,
      branding,
      features,
      defaults,
      window: windowConfig,

      isFeatureEnabled(feature: string): boolean {
        switch (feature) {
          case 'updates':
            return features.updatesEnabled;
          case 'costTracking':
            return features.costTrackingEnabled;
          case 'announcements':
            return features.announcementsEnabled;
          case 'configuration':
            return features.configurationEnabled;
          case 'telemetryUi':
            return features.telemetryUiEnabled;
          case 'showModelSelector':
            return features.showModelSelector !== false;
          default:
            return true;
        }
      },

      isNavItemEnabled(itemId: string): boolean {
        return navSet.has(itemId);
      },

      getNavLabel(itemId: string, defaultLabel: string): string {
        return features.navigationLabels?.[itemId] ?? defaultLabel;
      },

      isSettingsTabEnabled(tabId: string): boolean {
        return tabSet.has(tabId);
      },

      isSectionHidden(sectionId: string): boolean {
        return hiddenSections.has(sectionId);
      },

      isProviderAllowed(providerId: string): boolean {
        if (!allowedProviders) return true;
        return allowedProviders.has(providerId.toLowerCase());
      },

      getRandomGreeting(): string {
        const greetings = branding.greetings;
        if (greetings.length === 0) return "What's on your mind?";
        return greetings[Math.floor(Math.random() * greetings.length)];
      },
    };
  }, [config]);

  return <WhiteLabelContext.Provider value={value}>{children}</WhiteLabelContext.Provider>;
};

export function useWhiteLabel(): WhiteLabelContextType {
  const context = useContext(WhiteLabelContext);
  if (!context) {
    throw new Error('useWhiteLabel must be used within a WhiteLabelProvider');
  }
  return context;
}
