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

import { RESOURCES_PREFIX } from './constants';

function copyDirRecursive(src: string, dest: string): void {
  fs.mkdirSync(dest, { recursive: true });
  for (const entry of fs.readdirSync(src, { withFileTypes: true })) {
    const srcPath = path.join(src, entry.name);
    const destPath = path.join(dest, entry.name);
    if (entry.isDirectory()) {
      copyDirRecursive(srcPath, destPath);
    } else {
      fs.copyFileSync(srcPath, destPath);
    }
  }
}

/**
 * Bundle skills, tools, and other assets referenced by the whitelabel config
 * into src/whitelabel-resources/ so they get picked up by extraResource.
 * Rewrites paths in the config to use RESOURCES_PREFIX.
 */
function bundleAssets(config: WhiteLabelConfig, configDir: string, projectRoot: string): void {
  const resourcesDir = path.join(projectRoot, 'src', 'whitelabel-resources');

  // Clean previous bundle
  if (fs.existsSync(resourcesDir)) {
    fs.rmSync(resourcesDir, { recursive: true });
  }

  let needsBundle = false;

  // Bundle skill directories
  if (config.defaults.skills) {
    for (const skill of config.defaults.skills) {
      const resolved = path.resolve(configDir, skill.path);
      if (!fs.existsSync(resolved)) {
        console.warn(`[WhiteLabel] Skill path not found: ${resolved}`);
        continue;
      }
      const destName = path.basename(resolved);
      const dest = path.join(resourcesDir, 'skills', destName);
      console.log(`[WhiteLabel] Bundling skill: ${resolved} → ${dest}`);
      copyDirRecursive(resolved, dest);
      skill.path = `${RESOURCES_PREFIX}/skills/${destName}`;
      needsBundle = true;
    }
  }

  // Bundle tool binaries
  if (config.defaults.tools) {
    for (const tool of config.defaults.tools) {
      const resolved = path.resolve(configDir, tool.path);
      if (!fs.existsSync(resolved)) {
        console.warn(`[WhiteLabel] Tool binary not found: ${resolved}`);
        continue;
      }
      const destName = path.basename(resolved);
      const dest = path.join(resourcesDir, 'tools', destName);
      console.log(`[WhiteLabel] Bundling tool: ${resolved} → ${dest}`);
      fs.mkdirSync(path.dirname(dest), { recursive: true });
      fs.copyFileSync(resolved, dest);
      // Preserve executable permission
      fs.chmodSync(dest, 0o755);
      tool.path = `${RESOURCES_PREFIX}/tools/${destName}`;
      needsBundle = true;
    }
  }

  if (!needsBundle && fs.existsSync(resourcesDir)) {
    fs.rmSync(resourcesDir, { recursive: true });
  }
}

export function loadWhiteLabelConfig(projectRoot: string): WhiteLabelConfig {
  // Allow override via env var — resolve relative paths from CWD (not projectRoot)
  const envPath = process.env.WHITELABEL_CONFIG;
  const configPath = envPath
    ? path.resolve(process.cwd(), envPath)
    : path.join(projectRoot, 'whitelabel.yaml');

  if (!fs.existsSync(configPath)) {
    console.log('[WhiteLabel] No whitelabel.yaml found, using defaults');
    return DEFAULT_WHITELABEL_CONFIG;
  }

  console.log(`[WhiteLabel] Loading config from ${configPath}`);
  const raw = fs.readFileSync(configPath, 'utf-8');
  const parsed = yaml.parse(raw) as Partial<WhiteLabelConfig>;
  const config = deepMerge(DEFAULT_WHITELABEL_CONFIG, parsed) as WhiteLabelConfig;

  // Bundle assets referenced by the config (skills, tools)
  const configDir = path.dirname(configPath);
  bundleAssets(config, configDir, projectRoot);

  return config;
}
