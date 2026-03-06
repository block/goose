import type { TranslationSchema } from '../en';
import { app } from './app';
import { core } from './core';
import { integrations } from './integrations';
import { workspace } from './workspace';

export const zhCN: TranslationSchema = {
  ...core,
  ...app,
  ...workspace,
  ...integrations,
};
