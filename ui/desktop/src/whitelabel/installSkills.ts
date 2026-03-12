/**
 * Install whitelabel skills into the working directory's .goose/skills/
 * so the summon extension discovers them at runtime.
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
      // Preserve executable bit for scripts
      const mode = fsSync.statSync(srcPath).mode;
      fsSync.chmodSync(destPath, mode);
    }
  }
}

function expandHome(p: string): string {
  if (p.startsWith('~/')) {
    return path.join(os.homedir(), p.slice(2));
  }
  return p;
}

export function installWhiteLabelSkills(config: WhiteLabelConfig, resourcesPath: string): void {
  const skills = config.defaults?.skills;
  if (!skills || skills.length === 0) return;

  // Resolve working dir — create it if needed
  const workingDir = expandHome(config.defaults?.workingDir || os.homedir());
  fsSync.mkdirSync(workingDir, { recursive: true });

  const skillsDir = path.join(workingDir, '.goose', 'skills');
  fsSync.mkdirSync(skillsDir, { recursive: true });

  for (const skill of skills) {
    let skillSrc = skill.path;

    // Resolve __WHITELABEL_RESOURCES__ prefix
    if (skillSrc.startsWith(RESOURCES_PREFIX)) {
      const relative = skillSrc.slice(RESOURCES_PREFIX.length + 1);
      // Dev: resources are under ui/desktop/src/whitelabel-resources/
      // Packaged: resources are under process.resourcesPath/whitelabel-resources/
      const devPath = path.join(
        resourcesPath,
        'ui',
        'desktop',
        'src',
        'whitelabel-resources',
        relative
      );
      const packagedPath = path.join(resourcesPath, 'whitelabel-resources', relative);
      skillSrc = fsSync.existsSync(devPath) ? devPath : packagedPath;
    }

    if (!path.isAbsolute(skillSrc)) {
      skillSrc = path.resolve(process.cwd(), skillSrc);
    }

    if (!fsSync.existsSync(skillSrc)) {
      log.warn(`[whitelabel] Skill source not found: ${skillSrc}`);
      continue;
    }

    const skillName = path.basename(skillSrc);
    const dest = path.join(skillsDir, skillName);

    if (fsSync.existsSync(dest)) {
      fsSync.rmSync(dest, { recursive: true });
    }

    copyDirSync(skillSrc, dest);
    log.info(`[whitelabel] Installed skill: ${skillName} → ${dest}`);
  }
}
