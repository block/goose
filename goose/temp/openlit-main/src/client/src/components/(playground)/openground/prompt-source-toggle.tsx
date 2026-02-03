"use client";

import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Textarea } from "@/components/ui/textarea";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { useRootStore } from "@/store";
import { Component, FileTextIcon, LibraryIcon } from "lucide-react";
import { useEffect, useState } from "react";
import { Badge } from "@/components/ui/badge";
import VariableEditor from "./variable-editor";
import useFetchWrapper from "@/utils/hooks/useFetchWrapper";
import { toast } from "sonner";
import getMessage from "@/constants/messages";

interface Prompt {
	promptId: string;
	name: string;
	latestVersion: number;
}

interface PromptVersion {
	promptId: string;
	versionId: string;
	name: string;
	version: string;
	prompt: string;
	tags: string;
	metaProperties: string;
}

export default function PromptSourceToggle() {
	const promptSource = useRootStore((state) => state.openground.promptSource);
	const setPromptSource = useRootStore((state) => state.openground.setPromptSource);
	const { data: prompts, fireRequest, isLoading: loading } =
		useFetchWrapper<Prompt[]>();
	const { fireRequest: firePromptDetail, isLoading: loadingDetail } =
		useFetchWrapper<PromptVersion>();
	const [selectedPromptName, setSelectedPromptName] = useState<string>("");

	useEffect(() => {
		// Load prompts from Prompt Hub
		fireRequest({
			requestType: "POST",
			url: "/api/prompt/get",
			failureCb: (err?: string) => {
				toast.error(err || getMessage().OPENGROUND_LOAD_PROMPTS_FAILED, {
					id: "prompts",
				});
			},
		});
	}, []);

	const handleTabChange = (value: string) => {
		if (value === "custom") {
			setPromptSource({
				type: "custom",
				content: promptSource.type === "custom" ? promptSource.content : "",
				variables: {},
			});
		} else {
			setPromptSource({
				type: "prompt-hub",
				promptId: undefined,
				promptName: undefined,
				version: undefined,
				variables: {},
			});
			setSelectedPromptName("");
		}
	};

	const handleCustomPromptChange = (value: string) => {
		setPromptSource({
			...promptSource,
			type: "custom",
			content: value,
		});
	};

	const handlePromptSelect = (promptId: string) => {
		const selected = prompts?.find((p) => p.promptId === promptId);
		if (selected) {
			setSelectedPromptName(selected.name);

			// Fetch the full prompt content from the prompt detail API
			firePromptDetail({
				requestType: "GET",
				url: `/api/prompt/get/${selected.promptId}?version=${selected.latestVersion}`,
				responseDataKey: "data.[0]",
				successCb: (data: PromptVersion) => {
					if (data) {
						setPromptSource({
							type: "prompt-hub",
							promptId: selected.promptId,
							promptName: selected.name,
							version: parseInt(data.version),
							content: data.prompt,
							variables: {},
						});
					}
				},
				failureCb: (err?: string) => {
					toast.error(err || getMessage().OPENGROUND_LOAD_PROMPT_DETAILS_FAILED, {
						id: "prompt-detail",
					});
				},
			});
		}
	};

	return (
		<Card>
			<CardHeader className="pb-4">
				<CardTitle className="text-lg flex items-center gap-2">
					{getMessage().OPENGROUND_PROMPT_CONFIGURATION}
				</CardTitle>
			</CardHeader>
			<CardContent className="space-y-4">
				<Tabs
					value={promptSource.type}
					onValueChange={handleTabChange}
					className="w-full"
				>
					<TabsList className="grid w-full grid-cols-2">
						<TabsTrigger value="custom" className="gap-2">
							<FileTextIcon className="h-4 w-4" />
							{getMessage().OPENGROUND_CUSTOM}
						</TabsTrigger>
						<TabsTrigger value="prompt-hub" className="gap-2">
							<Component className="h-4 w-4" />
							{getMessage().OPENGROUND_PROMPT_HUB}
						</TabsTrigger>
					</TabsList>

					<TabsContent value="custom" className="space-y-4 mt-4">
						<div className="space-y-2">
							<Textarea
								placeholder={getMessage().OPENGROUND_ENTER_PROMPT_PLACEHOLDER}
								value={promptSource.content || ""}
								onChange={(e) => handleCustomPromptChange(e.target.value)}
								className="min-h-[120px] resize-none font-mono text-sm"
							/>
							<p className="text-xs text-stone-500 dark:text-stone-400">
								{getMessage().PROMPT_TIPS_TO_USE_VARIABLES}
							</p>
						</div>
						<VariableEditor promptText={promptSource.content || ""} />
					</TabsContent>

					<TabsContent value="prompt-hub" className="space-y-4 mt-4">
						<Select
							value={promptSource.promptId}
							onValueChange={handlePromptSelect}
							disabled={loading || loadingDetail}
						>
							<SelectTrigger className="w-full">
								<SelectValue
									placeholder={
										loading || loadingDetail
											? getMessage().LOADING
											: selectedPromptName || getMessage().OPENGROUND_SELECT_PROMPT
									}
								/>
							</SelectTrigger>
							<SelectContent>
								{(!prompts || prompts.length === 0) && !loading ? (
									<div className="p-2 text-sm text-stone-500 dark:text-stone-400">
										{getMessage().OPENGROUND_NO_PROMPTS_FOUND}
									</div>
								) : (
									prompts?.map((prompt) => (
										<SelectItem key={prompt.promptId} value={prompt.promptId}>
											{prompt.name}
										</SelectItem>
									))
								)}
							</SelectContent>
						</Select>

						{promptSource.promptId && promptSource.content && (
							<div className="space-y-4">
								<div>
									<div className="flex items-center justify-between mb-2">
										<span className="text-sm font-medium text-stone-700 dark:text-stone-300">
											{getMessage().PROMPT_PREVIEW}
										</span>
										<Badge variant="secondary" className="text-xs">
											v{promptSource.version}
										</Badge>
									</div>
									<div className="p-4 bg-stone-50 dark:bg-stone-900 rounded-lg border border-stone-200 dark:border-stone-800">
										<p className="text-sm font-mono text-stone-700 dark:text-stone-300 whitespace-pre-wrap leading-relaxed">
											{promptSource.content}
										</p>
									</div>
								</div>
								<VariableEditor promptText={promptSource.content || ""} />
							</div>
						)}

						{loadingDetail && (
							<div className="flex items-center justify-center py-8">
								<div className="flex flex-col items-center gap-2">
									<div className="h-8 w-8 animate-spin rounded-full border-2 border-stone-300 border-t-blue-600" />
									<span className="text-sm text-stone-600 dark:text-stone-400">
										{getMessage().OPENGROUND_LOADING_PROMPT_DETAILS}
									</span>
								</div>
							</div>
						)}
					</TabsContent>
				</Tabs>
			</CardContent>
		</Card>
	);
}
