import { app } from './app';
import { core } from './core';
import { integrations } from './integrations';
import { workspace } from './workspace';

export const en = {
  ...core,
  ...app,
  ...workspace,
  ...integrations,
};

export type TranslationSchema = typeof en;
