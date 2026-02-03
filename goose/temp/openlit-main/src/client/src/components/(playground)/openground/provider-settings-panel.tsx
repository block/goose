"use client";

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select";
import { Slider } from "@/components/ui/slider";
import { useRootStore } from "@/store";
import { Settings2Icon, XIcon, SettingsIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { useState } from "react";
import getMessage from "@/constants/messages";
import ModelManagementDialog from "./model-management-dialog";

export default function ProviderSettingsPanel() {
	const selectedProvidersNew = useRootStore((state) => state.openground.selectedProvidersNew);
	const availableProviders = useRootStore((state) => state.openground.availableProviders);
	const setProviderConfigNew = useRootStore((state) => state.openground.setProviderConfigNew);
	const updateProviderModel = useRootStore((state) => state.openground.updateProviderModel);
	const removeProviderNew = useRootStore((state) => state.openground.removeProviderNew);

	const [modelManagementOpen, setModelManagementOpen] = useState(false);
	const [selectedProviderForModelManagement, setSelectedProviderForModelManagement] = useState<string>("");
	const [customModelInputs, setCustomModelInputs] = useState<Record<number, boolean>>({});
	const [customModelValues, setCustomModelValues] = useState<Record<number, string>>({});

	if (selectedProvidersNew.length === 0) {
		return null;
	}

	const handleModelChange = (index: number, modelId: string) => {
		updateProviderModel(index, modelId);
		setCustomModelInputs({ ...customModelInputs, [index]: false });
	};

	const handleConfigChange = (index: number, key: string, value: number) => {
		setProviderConfigNew(index, { [key]: value });
	};

	const handleCustomModelToggle = (index: number) => {
		setCustomModelInputs({ ...customModelInputs, [index]: !customModelInputs[index] });
	};

	const handleCustomModelChange = (index: number, value: string) => {
		setCustomModelValues({ ...customModelValues, [index]: value });
		updateProviderModel(index, value);
	};

	const openModelManagement = (providerId: string) => {
		setSelectedProviderForModelManagement(providerId);
		setModelManagementOpen(true);
	};

	return (
		<Card className="border-stone-200 dark:border-stone-800">
			<CardHeader className="pb-3">
				<CardTitle className="text-lg flex items-center gap-2">
					<Settings2Icon className="h-5 w-5" />
					{getMessage().OPENGROUND_PROVIDER_SETTINGS} ({selectedProvidersNew.length})
				</CardTitle>
			</CardHeader>
			<CardContent className="space-y-4">
				{selectedProvidersNew.map((selectedProvider, index) => {
					const provider = availableProviders.find(
						(p) => p.providerId === selectedProvider.provider
					);

					if (!provider) return null;

					return (
						<Card key={index} className="border-stone-200 dark:border-stone-700">
							<CardHeader className="pb-3 flex-row items-center justify-between space-y-0">
								<div className="flex items-center gap-2">
									<div className="w-8 h-8 rounded bg-stone-100 dark:bg-stone-800 flex items-center justify-center text-sm font-bold text-stone-600 dark:text-stone-400">
										{provider.displayName.charAt(0)}
									</div>
									<div>
										<h4 className="font-medium text-sm text-stone-900 dark:text-stone-100">
											{provider.displayName}
										</h4>
										<p className="text-xs text-stone-500 dark:text-stone-400">
											{selectedProvider.model}
										</p>
									</div>
								</div>
								<Button
									variant="ghost"
									size="icon"
									className="h-8 w-8"
									onClick={() => removeProviderNew(index)}
								>
									<XIcon className="h-4 w-4" />
								</Button>
							</CardHeader>
							<CardContent className="space-y-4 pt-0">
								{/* Model Selection */}
								<div className="space-y-2">
									<div className="flex items-center justify-between">
										<Label htmlFor={`model-${index}`} className="text-xs">
											{getMessage().OPENGROUND_MODEL}
										</Label>
										<div className="flex gap-2">
											<Button
												variant="ghost"
												size="sm"
												className="h-6 text-xs px-2"
												onClick={() => handleCustomModelToggle(index)}
											>
												{customModelInputs[index] ? getMessage().OPENGROUND_USE_CUSTOM_MODEL : getMessage().OPENGROUND_OR_ENTER_CUSTOM}
											</Button>
											<Button
												variant="ghost"
												size="sm"
												className="h-6 text-xs px-2"
												onClick={() => openModelManagement(provider.providerId)}
											>
												<SettingsIcon className="h-3 w-3 mr-1" />
												{getMessage().OPENGROUND_MANAGE_MODELS}
											</Button>
										</div>
									</div>

									{customModelInputs[index] ? (
										<Input
											id={`custom-model-${index}`}
											placeholder={getMessage().OPENGROUND_ENTER_CUSTOM_MODEL_NAME}
											value={customModelValues[index] || ""}
											onChange={(e) => handleCustomModelChange(index, e.target.value)}
											className="h-9"
										/>
									) : (
										<Select
											value={selectedProvider.model}
											onValueChange={(value) => handleModelChange(index, value)}
										>
											<SelectTrigger id={`model-${index}`} className="h-auto">
												<SelectValue />
											</SelectTrigger>
											<SelectContent>
												{provider.supportedModels.map((model) => (
													<SelectItem key={model.id} value={model.id}>
														<div className="flex flex-col items-start gap-0.5">
															<span className="text-sm">{model.displayName}</span>
															<div className="flex gap-1.5 text-xs text-stone-500">
																<Badge variant="secondary" className="text-xs h-4 px-1">
																	{model.contextWindow.toLocaleString()}
																</Badge>
																<Badge variant="outline" className="text-xs h-4 px-1">
																	${model.inputPricePerMToken}/M
																</Badge>
															</div>
														</div>
													</SelectItem>
												))}
											</SelectContent>
										</Select>
									)}
								</div>

								{/* Temperature */}
								<div className="space-y-2">
									<div className="flex items-center justify-between">
										<Label htmlFor={`temperature-${index}`} className="text-xs">
											{getMessage().OPENGROUND_TEMPERATURE}
										</Label>
										<span className="text-xs text-stone-500 dark:text-stone-400">
											{selectedProvider.config.temperature ?? 1}
										</span>
									</div>
									<Slider
										id={`temperature-${index}`}
										min={0}
										max={2}
										step={0.1}
										value={[selectedProvider.config.temperature ?? 1]}
										onValueChange={([value]) =>
											handleConfigChange(index, "temperature", value)
										}
									/>
									<p className="text-xs text-stone-500 dark:text-stone-400">
										{getMessage().OPENGROUND_TEMPERATURE_DESCRIPTION}
									</p>
								</div>

								{/* Max Tokens */}
								<div className="space-y-2">
									<div className="flex items-center justify-between">
										<Label htmlFor={`maxTokens-${index}`} className="text-xs">
											{getMessage().OPENGROUND_MAX_TOKENS}
										</Label>
										<span className="text-xs text-stone-500 dark:text-stone-400">
											{selectedProvider.config.maxTokens ?? 1000}
										</span>
									</div>
									<Slider
										id={`maxTokens-${index}`}
										min={100}
										max={4000}
										step={100}
										value={[selectedProvider.config.maxTokens ?? 1000]}
										onValueChange={([value]) =>
											handleConfigChange(index, "maxTokens", value)
										}
									/>
									<p className="text-xs text-stone-500 dark:text-stone-400">
										{getMessage().OPENGROUND_MAX_TOKENS_DESCRIPTION}
									</p>
								</div>

								{/* Top P */}
								<div className="space-y-2">
									<div className="flex items-center justify-between">
										<Label htmlFor={`topP-${index}`} className="text-xs">
											{getMessage().OPENGROUND_TOP_P}
										</Label>
										<span className="text-xs text-stone-500 dark:text-stone-400">
											{selectedProvider.config.topP ?? 1}
										</span>
									</div>
									<Slider
										id={`topP-${index}`}
										min={0}
										max={1}
										step={0.05}
										value={[selectedProvider.config.topP ?? 1]}
										onValueChange={([value]) =>
											handleConfigChange(index, "topP", value)
										}
									/>
									<p className="text-xs text-stone-500 dark:text-stone-400">
										{getMessage().OPENGROUND_TOP_P_DESCRIPTION}
									</p>
								</div>
							</CardContent>
						</Card>
					);
				})}
			</CardContent>

			<ModelManagementDialog
				providerId={selectedProviderForModelManagement}
				open={modelManagementOpen}
				onOpenChange={setModelManagementOpen}
				onModelAdded={() => {
					// Optionally refresh provider list or models here
				}}
			/>
		</Card>
	);
}
