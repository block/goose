/**
 * Generate ZIP files for skills from Agent-Skills repository
 * 
 * This script creates ZIP files for each skill in the Agent-Skills repo
 * and outputs them to static/skills-data-zips/<skillId>.zip
 * 
 * Note: This script should run AFTER generate-skills-manifest.js
 * because it relies on the cloned repo being present in .tmp/agent-skills
 * 
 * Run this before building the documentation site.
 */

const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

// Configuration - must match generate-skills-manifest.js
const AGENT_SKILLS_REPO = 'https://github.com/block/Agent-Skills.git';
const TEMP_DIR = path.join(__dirname, '..', '.tmp');
const CLONED_REPO_DIR = path.join(TEMP_DIR, 'agent-skills');
const ZIPS_OUTPUT_DIR = path.join(__dirname, '..', 'static', 'skills-data-zips');

// Directories to skip when scanning for skills (not skill folders)
const SKIP_DIRS = ['.github', 'node_modules', '.git'];

/**
 * Clone the Agent-Skills repository if not already present
 */
function ensureRepoCloned() {
  if (fs.existsSync(CLONED_REPO_DIR)) {
    console.log('[generate-skills-zips] Agent-Skills repo already cloned');
    return true;
  }
  
  console.log('[generate-skills-zips] Cloning Agent-Skills repository...');
  
  // Create temp directory if it doesn't exist
  if (!fs.existsSync(TEMP_DIR)) {
    fs.mkdirSync(TEMP_DIR, { recursive: true });
  }
  
  try {
    execSync(`git clone --depth 1 ${AGENT_SKILLS_REPO} ${CLONED_REPO_DIR}`, {
      stdio: 'pipe',
      timeout: 60000
    });
    console.log('[generate-skills-zips] Successfully cloned Agent-Skills repository');
    return true;
  } catch (error) {
    console.error('[generate-skills-zips] ERROR: Failed to clone Agent-Skills repository');
    console.error('[generate-skills-zips] Error:', error.message);
    return false;
  }
}

/**
 * Check if a directory contains a SKILL.md file (i.e., is a skill folder)
 */
function isSkillDirectory(dirPath) {
  const skillMdPath = path.join(dirPath, 'SKILL.md');
  return fs.existsSync(skillMdPath);
}

/**
 * Clean up temporary files
 */
function cleanup() {
  if (fs.existsSync(CLONED_REPO_DIR)) {
    console.log('[generate-skills-zips] Cleaning up temporary files...');
    fs.rmSync(CLONED_REPO_DIR, { recursive: true, force: true });
  }
}

function generateSkillZips() {
  console.log('[generate-skills-zips] Starting...');
  
  // Ensure repo is cloned
  if (!ensureRepoCloned()) {
    console.error('[generate-skills-zips] Cannot generate ZIPs without Agent-Skills repo');
    process.exit(1);
  }
  
  // Create output directory if it doesn't exist
  if (!fs.existsSync(ZIPS_OUTPUT_DIR)) {
    fs.mkdirSync(ZIPS_OUTPUT_DIR, { recursive: true });
    console.log(`[generate-skills-zips] Created output directory: ${ZIPS_OUTPUT_DIR}`);
  }
  
  // Clean existing ZIPs
  const existingZips = fs.readdirSync(ZIPS_OUTPUT_DIR).filter(f => f.endsWith('.zip'));
  for (const zip of existingZips) {
    fs.unlinkSync(path.join(ZIPS_OUTPUT_DIR, zip));
  }
  console.log(`[generate-skills-zips] Cleaned ${existingZips.length} existing ZIP files`);
  
  // Get all skill directories from the cloned repo
  const entries = fs.readdirSync(CLONED_REPO_DIR, { withFileTypes: true });
  const skillDirs = entries
    .filter(d => d.isDirectory() && !SKIP_DIRS.includes(d.name))
    .map(d => d.name)
    .filter(name => isSkillDirectory(path.join(CLONED_REPO_DIR, name)));
  
  let generatedCount = 0;
  
  for (const skillId of skillDirs) {
    const skillDir = path.join(CLONED_REPO_DIR, skillId);
    const zipPath = path.join(ZIPS_OUTPUT_DIR, `${skillId}.zip`);
    
    try {
      // Use the system zip command to create the archive
      // cd into the cloned repo and zip the skill folder to preserve the folder name
      execSync(`cd "${CLONED_REPO_DIR}" && zip -r "${zipPath}" "${skillId}"`, {
        stdio: 'pipe'
      });
      
      const stats = fs.statSync(zipPath);
      console.log(`[generate-skills-zips] Created: ${skillId}.zip (${(stats.size / 1024).toFixed(1)} KB)`);
      generatedCount++;
    } catch (error) {
      console.error(`[generate-skills-zips] Error creating ZIP for ${skillId}:`, error.message);
    }
  }
  
  console.log(`[generate-skills-zips] Generated ${generatedCount} ZIP files in ${ZIPS_OUTPUT_DIR}`);
  
  // Clean up the cloned repo
  cleanup();
}

generateSkillZips();
