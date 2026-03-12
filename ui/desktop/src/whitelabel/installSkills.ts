/**
 * Install whitelabel skills to ~/.config/goose/skills/ so the summon extension
 * can discover them at runtime.
 *
 * At build time, the loader bundles skill directories into whitelabel-resources/
 * and rewrites paths to use __WHITELABEL_RESOURCES__. At runtime (dev mode),
 * skills may still have relative paths from the original YAML.
 *
 * This function resolves skill paths and copies them to the global skills dir.
 */

import fsSync from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import type { WhiteLabelConfig } from './types';
import { RESOURCES_PREFIX } from './constants';
import log from '../utils/logger';

function copyDirSync(src: string, dest: string): void {
  fsSync.mkdirSync(dest, { recursive: true });
  for (const entry of fsSync.readdirSync(src, { withFileTypes: true })) {
    const srcPath = path.join(src, entry.name);
    const destPath = path.join(dest, entry.name);
    if (entry.isDirectory()) {
      copyDirSync(srcPath, destPath);
    } else {
      fsSync.copyFileSync(srcPath, destPath);
    }
  }
}

export function installWhiteLabelSkills(
  config: WhiteLabelConfig,
  resourcesPath: string
): void {
  const skills = config.defaults?.skills;
  if (!skills || skills.length === 0) return;

  const gooseConfigDir = path.join(os.homedir(), '.config', 'goose', 'skills');
  fsSync.mkdirSync(gooseConfigDir, { recursive: true });

  for (const skill of skills) {
    let skillDir = skill.path;

    // Resolve __WHITELABEL_RESOURCES__ prefix
    // In dev mode, bundled resources are at ui/desktop/src/whitelabel-resources/
    // In packaged mode, they're at process.resourcesPath/whitelabel-resources/
    if (skillDir.startsWith(RESOURCES_PREFIX)) {
      const relative = skillDir.slice(RESOURCES_PREFIX.length + 1);
      const devPath = path.join(resourcesPath, 'ui', 'desktop', 'src', 'whitelabel-resources', relative);
      const packagedPath = path.join(resourcesPath, 'whitelabel-resources', relative);
      skillDir = fsSync.existsSync(devPath) ? devPath : packagedPath;
    }

    // Resolve relative paths (shouldn't happen — loader rewrites them)
    if (!path.isAbsolute(skillDir)) {
      skillDir = path.resolve(process.cwd(), skillDir);
    }

    if (!fsSync.existsSync(skillDir)) {
      log.warn(`[whitelabel] Skill directory not found: ${skillDir}`);
      continue;
    }

    const skillName = path.basename(skillDir);
    const dest = path.join(gooseConfigDir, skillName);

    // Always overwrite to keep skills fresh
    if (fsSync.existsSync(dest)) {
      fsSync.rmSync(dest, { recursive: true });
    }

    copyDirSync(skillDir, dest);
    log.info(`[whitelabel] Installed skill: ${skillName} → ${dest}`);
  }
}
