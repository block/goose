import React, { useState } from "react";
import Link from "@docusaurus/Link";
import { Check, Copy } from "lucide-react";
import type { Skill } from "@site/src/pages/skills/types";

function generateInstallCommand(repoUrl: string, skillId: string): string {
  return `npx skills add ${repoUrl} --skill ${skillId}`;
}

export function SkillCard({ skill }: { skill: Skill }) {
  const [copied, setCopied] = useState(false);

  const handleCopyInstall = (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();

    const command = generateInstallCommand(skill.repoUrl, skill.id);
    navigator.clipboard.writeText(command);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className="relative w-full h-full">
      <Link
        to={`/skills/detail?id=${skill.id}`}
        className="block no-underline hover:no-underline h-full"
      >
        <div className="absolute inset-0 rounded-2xl bg-purple-500 opacity-10 blur-2xl" />

        <div className="relative z-10 w-full h-full rounded-2xl border border-zinc-200 dark:border-zinc-700 bg-white dark:bg-[#1A1A1A] flex flex-col justify-between p-6 transition-shadow duration-200 ease-in-out hover:shadow-[0_0_0_2px_rgba(99,102,241,0.4),_0_4px_20px_rgba(99,102,241,0.1)]">
          <div className="space-y-4">
            {/* Header with name and badges */}
            <div className="flex items-start justify-between gap-2">
              <h3 className="font-semibold text-base text-zinc-900 dark:text-white leading-snug">
                {skill.name}
              </h3>
              <div className="flex gap-2 flex-shrink-0">
                {skill.isCommunity && (
                  <span className="inline-flex items-center h-6 px-2 rounded-full bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200 text-xs font-medium border border-yellow-200 dark:border-yellow-800">
                    Community
                  </span>
                )}
                {skill.version && (
                  <span className="inline-flex items-center h-6 px-2 rounded-full bg-zinc-100 text-zinc-600 dark:bg-zinc-800 dark:text-zinc-400 text-xs font-medium">
                    v{skill.version}
                  </span>
                )}
              </div>
            </div>

            {/* Description */}
            <p className="text-sm text-zinc-600 dark:text-zinc-400">
              {skill.description}
            </p>

            {/* Tags - show max 4 on card, rest visible on detail page */}
            {skill.tags.length > 0 && (
              <div className="flex flex-wrap gap-2">
                {skill.tags.slice(0, 4).map((tag, index) => (
                  <span
                    key={index}
                    className="inline-flex items-center h-7 px-3 rounded-full border border-zinc-300 bg-zinc-100 text-zinc-700 dark:border-zinc-700 dark:bg-zinc-900 dark:text-zinc-300 text-xs font-medium"
                  >
                    {tag}
                  </span>
                ))}
              </div>
            )}

            {/* Supporting files indicator */}
            {skill.supportingFilesType === 'scripts' && (
              <div className="text-xs text-zinc-500 dark:text-zinc-500">
                ⚙️ Runs scripts
              </div>
            )}
            {skill.supportingFilesType === 'templates' && (
              <div className="text-xs text-zinc-500 dark:text-zinc-500">
                📄 Includes templates
              </div>
            )}
            {skill.supportingFilesType === 'multi-file' && (
              <div className="text-xs text-zinc-500 dark:text-zinc-500">
                📁 Multi-file skill
              </div>
            )}
          </div>

          {/* Footer with actions */}
          <div className="flex flex-col gap-3 pt-6 mt-2 border-t border-zinc-100 dark:border-zinc-800">
            {/* Install command display */}
            <div
              onClick={handleCopyInstall}
              className="flex items-center gap-2 -mx-2 px-2 py-1 rounded cursor-pointer hover:bg-zinc-100 dark:hover:bg-zinc-800 transition-colors group"
            >
              <code className="flex-1 text-xs font-mono text-zinc-600 dark:text-zinc-400 truncate">
                <span className="text-zinc-400 dark:text-zinc-500">$</span> {generateInstallCommand(skill.repoUrl, skill.id)}
              </code>
              <span className="flex-shrink-0 text-zinc-400 dark:text-zinc-500 group-hover:text-zinc-600 dark:group-hover:text-zinc-300 transition-colors">
                {copied ? (
                  <Check className="h-4 w-4 text-green-600 dark:text-green-400" />
                ) : (
                  <Copy className="h-4 w-4" />
                )}
              </span>
            </div>

            {/* View Details and Author */}
            <div className="flex justify-between items-center">
              <span className="text-sm font-medium text-purple-600 dark:text-purple-400">
                View Details →
              </span>

              {/* Author */}
              {skill.author && (
                <span className="text-sm text-zinc-500 dark:text-zinc-400">
                  by {skill.author}
                </span>
              )}
            </div>
          </div>
        </div>
      </Link>
    </div>
  );
}

export type { Skill };
