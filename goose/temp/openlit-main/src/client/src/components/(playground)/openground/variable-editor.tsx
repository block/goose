"use client";

import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useRootStore } from "@/store";
import { Code2Icon } from "lucide-react";
import { useMemo } from "react";
import getMessage from "@/constants/messages";

interface VariableEditorProps {
	promptText: string;
}

export default function VariableEditor({ promptText }: VariableEditorProps) {
	const promptSource = useRootStore((state) => state.openground.promptSource);
	const setPromptVariable = useRootStore((state) => state.openground.setPromptVariable);

	// Extract variables from prompt text
	const variables = useMemo(() => {
		const regex = /\{\{([^}]+)\}\}/g;
		const matches: string[] = [];
		let match;

		while ((match = regex.exec(promptText)) !== null) {
			const varName = match[1].trim();
			if (!matches.includes(varName)) {
				matches.push(varName);
			}
		}

		return matches;
	}, [promptText]);

	if (variables.length === 0) {
		return null;
	}

	return (
		<div className="space-y-3 pt-4 border-t border-stone-200 dark:border-stone-800">
			<div className="flex items-center gap-2 text-sm font-medium text-stone-700 dark:text-stone-300">
				<Code2Icon className="h-4 w-4" />
				{getMessage().VARIABLES}
			</div>
			<div className="grid gap-3">
				{variables.map((variable) => (
					<div key={variable} className="space-y-1.5">
						<Label htmlFor={`var-${variable}`} className="text-sm">
							{variable}
						</Label>
						<Input
							id={`var-${variable}`}
							placeholder={`${getMessage().OPENGROUND_ENTER_VALUE_FOR} ${variable}`}
							value={promptSource.variables?.[variable] || ""}
							onChange={(e) => setPromptVariable(variable, e.target.value)}
							className="font-mono text-sm"
						/>
					</div>
				))}
			</div>
			<p className="text-xs text-stone-500 dark:text-stone-400">
				{getMessage().OPENGROUND_VARIABLES_SUBSTITUTED_INFO}
			</p>
		</div>
	);
}
