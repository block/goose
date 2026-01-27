import React, { useState } from "react";
import Link from "@docusaurus/Link";
import { Check } from "lucide-react";
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
                {skill.isExternal && (
                  <span className="inline-flex items-center h-6 px-2 rounded-full bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200 text-xs font-medium border border-blue-200 dark:border-blue-800">
                    External
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

            {/* Tags */}
            {skill.tags.length > 0 && (
              <div className="flex flex-wrap gap-2">
                {skill.tags.map((tag, index) => (
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
            {skill.hasSupporting && (
              <div className="text-xs text-zinc-500 dark:text-zinc-500">
                📁 Includes supporting files
              </div>
            )}
          </div>

          {/* Footer with actions */}
          <div className="flex justify-between items-center pt-6 mt-2 border-t border-zinc-100 dark:border-zinc-800">
            {/* Install button */}
            <div className="relative group">
              <button
                onClick={handleCopyInstall}
                className={`text-sm font-medium px-3 py-1 rounded cursor-pointer flex items-center gap-1.5 transition-colors ${
                  copied
                    ? "bg-green-100 text-green-700 dark:bg-green-900 dark:text-green-300"
                    : "text-zinc-700 bg-zinc-200 dark:bg-zinc-700 dark:text-white dark:hover:bg-zinc-600 hover:bg-zinc-300"
                }`}
              >
                {copied ? (
                  <>
                    <Check className="h-3.5 w-3.5" />
                    Copied!
                  </>
                ) : (
                  "Copy Install"
                )}
              </button>

            </div>

            {/* View Source link - always show, links to Agent-Skills repo */}
            <a
              href={skill.viewSourceUrl}
              target="_blank"
              rel="noopener noreferrer"
              className="text-sm font-medium text-purple-600 hover:underline dark:text-purple-400"
              onClick={(e) => e.stopPropagation()}
            >
              View Source →
            </a>

            {/* Author */}
            {skill.author && (
              <span className="text-sm text-zinc-500 dark:text-zinc-400">
                by {skill.author}
              </span>
            )}
          </div>
        </div>
      </Link>
    </div>
  );
}

export type { Skill };
