import React from 'react';
import Layout from '@docusaurus/theme-classic/lib/theme/Layout';
import CodeBlock from '@docusaurus/theme-classic/lib/theme/CodeBlock';

/**
 * Skill status indicator
 */
export type SkillStatus = 'experimental' | 'stable';

/**
 * Install method for a skill
 * - 'npx-single': npx skills add <owner>/<repo>
 * - 'npx-multi': npx skills add <url> --skill <name>
 * - 'download': No repo, show download button
 */
export type SkillInstallMethod = 'npx-single' | 'npx-multi' | 'download';

/**
 * Supporting files type - indicates what kind of extra files the skill includes
 * - 'scripts': Contains executable files (.sh, .py, .js, etc.)
 * - 'templates': Contains template files (.template., .example., etc.)
 * - 'multi-file': Contains other supporting files
 * - 'none': No supporting files
 */
export type SupportingFilesType = 'scripts' | 'templates' | 'multi-file' | 'none';

/**
 * Skill type definition
 */
export type Skill = {
  id: string;                    // Derived from directory name
  name: string;                  // From frontmatter (required)
  description: string;           // From frontmatter (required)
  author?: string;               // From frontmatter
  version?: string;              // From frontmatter
  status: SkillStatus;           // From frontmatter (default: 'stable')
  tags: string[];                // From frontmatter (default: [])
  sourceUrl?: string;            // From frontmatter - optional external source URL
  content: string;               // Markdown content after frontmatter
  hasSupporting: boolean;        // Computed: has files beyond SKILL.md
  supportingFiles: string[];     // Computed: list of supporting file paths
  supportingFilesType: SupportingFilesType; // Computed: type of supporting files
  installMethod: SkillInstallMethod; // Computed based on source
  installCommand?: string;       // Computed: npx command
  viewSourceUrl: string;         // Computed: GitHub link to skill source
  repoUrl: string;               // Repository URL (Agent-Skills for official, sourceUrl for external)
  isCommunity: boolean;          // True if author is not "goose" (community-contributed)
};

/**
 * Filter group for sidebar
 */
export type SkillFilterGroup = {
  title: string;
  options: { label: string; value: string; count?: number }[];
};

/**
 * Types documentation page
 */
const SkillTypes: React.FC = () => {
  return (
    <Layout title="Skill Types" description="Type definitions for the Skills Marketplace">
      <div className="container margin-vert--lg">
        <h1>Skill Type Definitions</h1>
        <p>This page contains the type definitions used in the Skills Marketplace.</p>
        
        <h2>Skill Status</h2>
        <CodeBlock language="typescript">
{`type SkillStatus = 'experimental' | 'stable';`}
        </CodeBlock>

        <h2>Skill Install Method</h2>
        <CodeBlock language="typescript">
{`// Install method for a skill
// - 'npx-single': npx skills add <owner>/<repo>
// - 'npx-multi': npx skills add <url> --skill <name>
// - 'download': No repo, show download button
type SkillInstallMethod = 'npx-single' | 'npx-multi' | 'download';`}
        </CodeBlock>

        <h2>Skill</h2>
        <CodeBlock language="typescript">
{`type Skill = {
  id: string;                    // Derived from directory name
  name: string;                  // From frontmatter (required)
  description: string;           // From frontmatter (required)
  author?: string;               // From frontmatter
  version?: string;              // From frontmatter
  status: SkillStatus;           // From frontmatter (default: 'stable')
  tags: string[];                // From frontmatter (default: [])
  sourceUrl?: string;            // From frontmatter - optional external source URL
  content: string;               // Markdown content after frontmatter
  hasSupporting: boolean;        // Computed: has files beyond SKILL.md
  supportingFiles: string[];     // Computed: list of supporting file paths
  installMethod: SkillInstallMethod; // Computed based on source
  installCommand?: string;       // Computed: npx command
  viewSourceUrl: string;         // Computed: GitHub link to skill source
  repoUrl: string;               // Repository URL (Agent-Skills for official, sourceUrl for external)
  isCommunity: boolean;          // True if author is not "goose" (community-contributed)
};`}
        </CodeBlock>

        <h2>SKILL.md Frontmatter Schema</h2>
        <CodeBlock language="yaml">
{`---
# Required fields
name: string           # Skill identifier
description: string    # Brief description (1-2 sentences)

# Optional fields
author: string                    # Author name or GitHub handle
version: string                   # Semantic version (e.g., "1.0")
status: experimental | stable     # Development status (default: stable)
tags:                             # Array of category tags
  - string
source_url: string                # GitHub repo URL for npx install
---`}
        </CodeBlock>
      </div>
    </Layout>
  );
};

export default SkillTypes;
