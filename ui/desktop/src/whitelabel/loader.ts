/**
 * Build-time loader for whitelabel.yaml.
 * Used by the Vite plugin to read and merge config at build time.
 * This file runs in Node.js context (not browser).
 */
import * as fs from 'node:fs';
import * as path from 'node:path';
import * as yaml from 'yaml';
import type { WhiteLabelConfig } from './types';
import { DEFAULT_WHITELABEL_CONFIG } from './defaults';

/* eslint-disable @typescript-eslint/no-explicit-any */
function deepMerge(target: any, source: any): any {
  if (!source) return target;
  const result = { ...target };
  for (const key of Object.keys(source)) {
    const sourceVal = source[key];
    const targetVal = target[key];
    if (
      sourceVal !== undefined &&
      sourceVal !== null &&
      typeof sourceVal === 'object' &&
      !Array.isArray(sourceVal) &&
      typeof targetVal === 'object' &&
      !Array.isArray(targetVal) &&
      targetVal !== null
    ) {
      result[key] = deepMerge(targetVal, sourceVal);
    } else if (sourceVal !== undefined) {
      result[key] = sourceVal;
    }
  }
  return result;
}
/* eslint-enable @typescript-eslint/no-explicit-any */

export function loadWhiteLabelConfig(projectRoot: string): WhiteLabelConfig {
  // Allow override via env var
  const configPath = process.env.WHITELABEL_CONFIG || path.join(projectRoot, 'whitelabel.yaml');

  if (!fs.existsSync(configPath)) {
    console.log('[WhiteLabel] No whitelabel.yaml found, using defaults');
    return DEFAULT_WHITELABEL_CONFIG;
  }

  console.log(`[WhiteLabel] Loading config from ${configPath}`);
  const raw = fs.readFileSync(configPath, 'utf-8');
  const parsed = yaml.parse(raw) as Partial<WhiteLabelConfig>;

  return deepMerge(DEFAULT_WHITELABEL_CONFIG, parsed) as WhiteLabelConfig;
}
