/**
 * Generate skills manifest from Agent-Skills repository
 * 
 * This script clones the block/Agent-Skills repository and reads all SKILL.md files
 * to generate a skills-manifest.json file that the frontend can fetch.
 * 
 * It also supports external skills defined in a local external-skills.json file.
 * 
 * Run this before building the documentation site.
 */

const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');
const matter = require('gray-matter');

// Configuration
const AGENT_SKILLS_REPO = 'https://github.com/block/Agent-Skills.git';
const AGENT_SKILLS_REPO_URL = 'https://github.com/block/Agent-Skills';
const TEMP_DIR = path.join(__dirname, '..', '.tmp');
const CLONED_REPO_DIR = path.join(TEMP_DIR, 'agent-skills');
const MANIFEST_OUTPUT = path.join(__dirname, '..', 'static', 'skills-manifest.json');
const EXTERNAL_SKILLS_FILE = path.join(__dirname, '..', 'static', 'external-skills.json');

// Directories to skip when scanning for skills (not skill folders)
const SKIP_DIRS = ['.github', 'node_modules', '.git'];

/**
 * Clone or update the Agent-Skills repository
 */
function cloneAgentSkillsRepo() {
  console.log('[generate-skills-manifest] Fetching Agent-Skills repository...');
  
  // Create temp directory if it doesn't exist
  if (!fs.existsSync(TEMP_DIR)) {
    fs.mkdirSync(TEMP_DIR, { recursive: true });
  }
  
  // Remove existing clone if present
  if (fs.existsSync(CLONED_REPO_DIR)) {
    console.log('[generate-skills-manifest] Removing existing clone...');
    fs.rmSync(CLONED_REPO_DIR, { recursive: true, force: true });
  }
  
  // Shallow clone the repository
  try {
    execSync(`git clone --depth 1 ${AGENT_SKILLS_REPO} ${CLONED_REPO_DIR}`, {
      stdio: 'pipe',
      timeout: 60000 // 60 second timeout
    });
    console.log('[generate-skills-manifest] Successfully cloned Agent-Skills repository');
  } catch (error) {
    console.error('[generate-skills-manifest] ERROR: Failed to clone Agent-Skills repository');
    console.error('[generate-skills-manifest] Error:', error.message);
    throw new Error('Failed to fetch Agent-Skills repository. Build cannot continue.');
  }
}

/**
 * Clean up temporary files
 */
function cleanup() {
  if (fs.existsSync(CLONED_REPO_DIR)) {
    console.log('[generate-skills-manifest] Cleaning up temporary files...');
    fs.rmSync(CLONED_REPO_DIR, { recursive: true, force: true });
  }
}

/**
 * Determine install method based on source configuration
 */
function determineInstallMethod(isExternal, sourceUrl) {
  if (isExternal && sourceUrl) {
    // External skill with a source URL
    const simpleRepoPattern = /^https:\/\/github\.com\/[^\/]+\/[^\/]+\/?$/;
    if (simpleRepoPattern.test(sourceUrl)) {
      return 'npx-single';
    }
    return 'npx-multi';
  }
  // Official skill from Agent-Skills repo
  return 'npx-multi';
}

/**
 * Generate install command based on method and source
 */
function generateInstallCommand(skillId, isExternal, sourceUrl) {
  if (isExternal && sourceUrl) {
    const simpleRepoPattern = /^https:\/\/github\.com\/[^\/]+\/[^\/]+\/?$/;
    if (simpleRepoPattern.test(sourceUrl)) {
      const match = sourceUrl.match(/github\.com\/([^\/]+\/[^\/]+)/);
      if (match) {
        return `npx skills add ${match[1]}`;
      }
    }
    return `npx skills add ${sourceUrl} --skill ${skillId}`;
  }
  // Official skill from Agent-Skills repo
  return `npx skills add ${AGENT_SKILLS_REPO_URL} --skill ${skillId}`;
}

/**
 * Generate view source URL for a skill
 */
function generateViewSourceUrl(skillId, isExternal, sourceUrl) {
  if (isExternal && sourceUrl) {
    return sourceUrl;
  }
  // Official skill from Agent-Skills repo
  return `${AGENT_SKILLS_REPO_URL}/tree/main/${skillId}`;
}

/**
 * Get supporting files in a skill directory (excluding SKILL.md)
 */
function getSupportingFiles(skillDir) {
  const files = [];
  
  function walkDir(dir, prefix = '') {
    const entries = fs.readdirSync(dir, { withFileTypes: true });
    for (const entry of entries) {
      const fullPath = path.join(dir, entry.name);
      const relativePath = prefix ? `${prefix}/${entry.name}` : entry.name;
      
      if (entry.isDirectory()) {
        walkDir(fullPath, relativePath);
      } else if (entry.name !== 'SKILL.md') {
        files.push(relativePath);
      }
    }
  }
  
  walkDir(skillDir);
  return files;
}

/**
 * Determine the supporting files type based on file contents
 * Returns: 'scripts' | 'templates' | 'multi-file' | 'none'
 */
function determineSupportingFilesType(supportingFiles) {
  if (supportingFiles.length === 0) {
    return 'none';
  }

  // Executable file extensions
  const executableExtensions = ['.sh', '.bash', '.zsh', '.ps1', '.bat', '.cmd', '.py', '.rb', '.js', '.mjs', '.ts'];
  
  // Template-like patterns (file names or extensions)
  const templatePatterns = [
    /\.template\./i,
    /\.tmpl\./i,
    /\.tpl\./i,
    /template\./i,
    /\.example\./i,
    /\.sample\./i,
    /\.skeleton\./i,
    /\.stub\./i,
    /\.j2$/i,
    /\.jinja2?$/i,
    /\.mustache$/i,
    /\.hbs$/i,
    /\.handlebars$/i,
    /\.ejs$/i,
    /\.erb$/i,
  ];

  const hasExecutable = supportingFiles.some(file => {
    const ext = path.extname(file).toLowerCase();
    return executableExtensions.includes(ext);
  });

  if (hasExecutable) {
    return 'scripts';
  }

  const hasTemplates = supportingFiles.some(file => {
    return templatePatterns.some(pattern => pattern.test(file));
  });

  if (hasTemplates) {
    return 'templates';
  }

  return 'multi-file';
}

/**
 * Check if a directory contains a SKILL.md file (i.e., is a skill folder)
 */
function isSkillDirectory(dirPath) {
  const skillMdPath = path.join(dirPath, 'SKILL.md');
  return fs.existsSync(skillMdPath);
}

/**
 * Process official skills from the cloned Agent-Skills repo
 */
function processOfficialSkills() {
  const skills = [];
  
  // Get all directories in the cloned repo
  const entries = fs.readdirSync(CLONED_REPO_DIR, { withFileTypes: true });
  
  for (const entry of entries) {
    // Skip non-directories and special directories
    if (!entry.isDirectory() || SKIP_DIRS.includes(entry.name)) {
      continue;
    }
    
    const skillId = entry.name;
    const skillDir = path.join(CLONED_REPO_DIR, skillId);
    
    // Skip if not a skill directory (no SKILL.md)
    if (!isSkillDirectory(skillDir)) {
      continue;
    }
    
    const skillMdPath = path.join(skillDir, 'SKILL.md');
    
    try {
      const rawContent = fs.readFileSync(skillMdPath, 'utf8');
      const parsed = matter(rawContent);
      const frontmatter = parsed.data || {};
      const content = parsed.content || '';
      
      const supportingFiles = getSupportingFiles(skillDir);
      const sourceUrl = frontmatter.source_url || frontmatter.sourceUrl;
      const author = frontmatter.author;
      const isCommunity = author && author.toLowerCase() !== 'goose';
      
      const supportingFilesType = determineSupportingFilesType(supportingFiles);
      
      const skill = {
        id: skillId,
        name: frontmatter.name || skillId,
        description: frontmatter.description || 'No description provided.',
        author,
        version: frontmatter.version,
        tags: Array.isArray(frontmatter.tags) ? frontmatter.tags : [],
        sourceUrl, // Optional external source if skill references another repo
        content,
        hasSupporting: supportingFiles.length > 0,
        supportingFiles,
        supportingFilesType,
        installMethod: determineInstallMethod(false, sourceUrl),
        installCommand: generateInstallCommand(skillId, false, sourceUrl),
        viewSourceUrl: generateViewSourceUrl(skillId, false, sourceUrl),
        repoUrl: AGENT_SKILLS_REPO_URL,
        isCommunity,
      };
      
      skills.push(skill);
      console.log(`[generate-skills-manifest] Processed official skill: ${skillId}`);
    } catch (error) {
      console.error(`[generate-skills-manifest] Error processing ${skillId}:`, error.message);
    }
  }
  
  return skills;
}

/**
 * Process external skills from external-skills.json
 */
function processExternalSkills() {
  const skills = [];
  
  if (!fs.existsSync(EXTERNAL_SKILLS_FILE)) {
    console.log('[generate-skills-manifest] No external-skills.json found, skipping external skills');
    return skills;
  }
  
  try {
    const externalData = JSON.parse(fs.readFileSync(EXTERNAL_SKILLS_FILE, 'utf8'));
    const externalSkills = externalData.skills || [];
    
    for (const extSkill of externalSkills) {
      const skillId = extSkill.id;
      const sourceUrl = extSkill.sourceUrl || extSkill.source_url;
      const author = extSkill.author;
      const isCommunity = author && author.toLowerCase() !== 'goose';
      
      const skill = {
        id: skillId,
        name: extSkill.name || skillId,
        description: extSkill.description || 'No description provided.',
        author,
        version: extSkill.version,
        tags: Array.isArray(extSkill.tags) ? extSkill.tags : [],
        sourceUrl,
        content: extSkill.content || '', // External skills may not have content
        hasSupporting: false,
        supportingFiles: [],
        supportingFilesType: 'none',
        installMethod: determineInstallMethod(true, sourceUrl),
        installCommand: generateInstallCommand(skillId, true, sourceUrl),
        viewSourceUrl: generateViewSourceUrl(skillId, true, sourceUrl),
        repoUrl: sourceUrl,
        isCommunity,
      };
      
      skills.push(skill);
      console.log(`[generate-skills-manifest] Processed external skill: ${skillId}`);
    }
  } catch (error) {
    console.error('[generate-skills-manifest] Error processing external skills:', error.message);
  }
  
  return skills;
}

/**
 * Main function to generate the manifest
 */
function generateManifest() {
  console.log('[generate-skills-manifest] Starting...');
  
  try {
    // Clone the Agent-Skills repository
    cloneAgentSkillsRepo();
    
    // Process official skills from the cloned repo
    const officialSkills = processOfficialSkills();
    
    // Process external skills from local JSON file
    const externalSkills = processExternalSkills();
    
    // Combine all skills
    const allSkills = [...officialSkills, ...externalSkills];
    
    // Check if we have any skills
    if (allSkills.length === 0) {
      console.error('[generate-skills-manifest] ERROR: No skills found. Build cannot continue.');
      throw new Error('No skills found in Agent-Skills repository.');
    }
    
    // Generate manifest
    const manifest = {
      skills: allSkills,
      generatedAt: new Date().toISOString(),
      count: allSkills.length,
      officialCount: officialSkills.length,
      externalCount: externalSkills.length,
      sourceRepo: AGENT_SKILLS_REPO_URL,
    };
    
    // Write manifest
    fs.writeFileSync(MANIFEST_OUTPUT, JSON.stringify(manifest, null, 2));
    console.log(`[generate-skills-manifest] Generated manifest with ${allSkills.length} skills (${officialSkills.length} official, ${externalSkills.length} external): ${MANIFEST_OUTPUT}`);
    
  } finally {
    // Always clean up
    cleanup();
  }
}

// Run the script
generateManifest();
