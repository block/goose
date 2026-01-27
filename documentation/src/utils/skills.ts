import type { Skill, SkillStatus, SkillInstallMethod } from "@site/src/pages/skills/types";
import matter from "gray-matter";

// Skills data is loaded from a generated JSON manifest at build time
// The manifest is created by scripts/generate-skills-manifest.js which:
// 1. Clones the block/Agent-Skills repository
// 2. Reads SKILL.md files from each skill folder
// 3. Optionally includes external skills from static/external-skills.json
// 4. Generates static/skills-manifest.json

// Cache for loaded skills
let skillsCache: Skill[] | null = null;
let skillsPromise: Promise<Skill[]> | null = null;

/**
 * Get a skill by its ID
 */
export function getSkillById(id: string): Skill | null {
  const allSkills = loadAllSkillsSync();
  return allSkills.find((skill) => skill.id === id) || null;
}

/**
 * Search skills by query string
 * Searches name, description, and tags
 */
export async function searchSkills(query: string): Promise<Skill[]> {
  const allSkills = await loadAllSkills();

  if (!query) return allSkills;

  const lowerQuery = query.toLowerCase();
  return allSkills.filter((skill) =>
    skill.name?.toLowerCase().includes(lowerQuery) ||
    skill.description?.toLowerCase().includes(lowerQuery) ||
    skill.tags?.some((tag) => tag.toLowerCase().includes(lowerQuery))
  );
}

/**
 * Load all skills - async version that fetches from manifest
 */
export async function loadAllSkills(): Promise<Skill[]> {
  if (skillsCache) return skillsCache;
  
  if (skillsPromise) return skillsPromise;
  
  skillsPromise = fetchSkillsManifest();
  skillsCache = await skillsPromise;
  return skillsCache;
}

/**
 * Load all skills synchronously (uses cache, returns empty if not loaded)
 */
export function loadAllSkillsSync(): Skill[] {
  if (skillsCache) return skillsCache;
  
  // Trigger async load
  loadAllSkills();
  
  // Return empty array for now - will be populated on next render
  return [];
}

/**
 * Fetch skills manifest from static files
 */
async function fetchSkillsManifest(): Promise<Skill[]> {
  try {
    // Fetch the pre-generated manifest
    const response = await fetch('/goose/skills-manifest.json');
    if (!response.ok) {
      console.error('Failed to fetch skills manifest:', response.status);
      return [];
    }
    
    const manifest = await response.json();
    return manifest.skills || [];
  } catch (error) {
    console.error('Error loading skills manifest:', error);
    return [];
  }
}

/**
 * Parse SKILL.md content into frontmatter and markdown content using gray-matter
 */
export function parseSkillMarkdown(content: string): { frontmatter: Record<string, any>; content: string } {
  try {
    const parsed = matter(content);
    return {
      frontmatter: parsed.data || {},
      content: parsed.content || ''
    };
  } catch (error) {
    console.error('Error parsing skill markdown:', error);
    return { frontmatter: {}, content };
  }
}

/**
 * Normalize raw frontmatter data to Skill type
 */
export function normalizeSkill(
  parsed: { frontmatter: Record<string, any>; content: string },
  id: string,
  supportingFiles: string[]
): Skill {
  const { frontmatter, content } = parsed;
  
  // Determine install method based on source_url
  const sourceUrl = frontmatter.source_url || frontmatter.sourceUrl;
  const installMethod = determineInstallMethod(sourceUrl, id);
  const installCommand = generateInstallCommand(sourceUrl, id, installMethod);
  
  return {
    id,
    name: frontmatter.name || id,
    description: frontmatter.description || 'No description provided.',
    author: frontmatter.author,
    version: frontmatter.version,
    status: (frontmatter.status as SkillStatus) || 'stable',
    tags: Array.isArray(frontmatter.tags) ? frontmatter.tags : [],
    sourceUrl,
    content,
    hasSupporting: supportingFiles.length > 0,
    supportingFiles,
    installMethod,
    installCommand,
    viewSourceUrl: generateViewSourceUrl(id),
  };
}

/**
 * Determine the install method based on source URL
 */
function determineInstallMethod(sourceUrl: string | undefined, skillId: string): SkillInstallMethod {
  if (!sourceUrl) {
    return 'download';
  }
  
  // If source URL contains a path to a specific skill, use npx-multi
  // Pattern: https://github.com/owner/repo with --skill needed
  // For skills in the goose repo, always use npx-multi since there are multiple skills
  if (sourceUrl.includes('block/goose')) {
    return 'npx-multi';
  }
  
  // For external repos that are single-skill repos, use npx-single
  // This is a heuristic - if the URL is just owner/repo format
  const simpleRepoPattern = /^https:\/\/github\.com\/[^\/]+\/[^\/]+\/?$/;
  if (simpleRepoPattern.test(sourceUrl)) {
    return 'npx-single';
  }
  
  // Default to npx-multi for safety
  return 'npx-multi';
}

/**
 * Generate the install command based on method
 */
function generateInstallCommand(
  sourceUrl: string | undefined,
  skillId: string,
  method: SkillInstallMethod
): string | undefined {
  if (method === 'download' || !sourceUrl) {
    return undefined;
  }
  
  if (method === 'npx-single') {
    // Extract owner/repo from URL
    const match = sourceUrl.match(/github\.com\/([^\/]+\/[^\/]+)/);
    if (match) {
      return `npx skills add ${match[1]}`;
    }
  }
  
  if (method === 'npx-multi') {
    return `npx skills add ${sourceUrl} --skill ${skillId}`;
  }
  
  return undefined;
}

/**
 * Generate the view source URL for a skill in the Agent-Skills repo
 */
function generateViewSourceUrl(skillId: string): string {
  return `https://github.com/block/Agent-Skills/tree/main/${skillId}`;
}

/**
 * Get all unique tags from all skills (async)
 */
export async function getAllTags(): Promise<string[]> {
  const allSkills = await loadAllSkills();
  const tagSet = new Set<string>();
  
  allSkills.forEach((skill) => {
    skill.tags.forEach((tag) => tagSet.add(tag));
  });
  
  return Array.from(tagSet).sort();
}
