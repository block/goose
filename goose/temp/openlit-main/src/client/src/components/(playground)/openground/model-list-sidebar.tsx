"use client";

import { useState, useMemo } from "react";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { SearchIcon, PlusIcon, ChevronDownIcon, ChevronRightIcon } from "lucide-react";
import getMessage from "@/constants/messages";
import { ProviderMetadata, ModelMetadata } from "@/types/openground";
import { cn } from "@/lib/utils";

interface CustomModel extends ModelMetadata {
	id: string; // UUID from database
	model_id: string; // Model identifier like "gpt-4o"
	provider?: string;
}

interface ModelListSidebarProps {
	providers: ProviderMetadata[];
	customModels: Record<string, CustomModel[]>;
	loading: boolean;
	selectedModel: ModelMetadata | null;
	selectedProvider: string | null;
	selectedIsCustom: boolean;
	onSelectModel: (model: ModelMetadata, provider: string, isCustom: boolean) => void;
	onAddNew: (provider: string) => void;
}

export default function ModelListSidebar({
	providers,
	customModels,
	loading,
	selectedModel,
	selectedProvider,
	selectedIsCustom,
	onSelectModel,
	onAddNew,
}: ModelListSidebarProps) {
	const [searchQuery, setSearchQuery] = useState("");
	const [expandedProviders, setExpandedProviders] = useState<Set<string>>(new Set());

	const toggleProvider = (providerId: string) => {
		const newExpanded = new Set(expandedProviders);
		if (newExpanded.has(providerId)) {
			newExpanded.delete(providerId);
		} else {
			newExpanded.add(providerId);
		}
		setExpandedProviders(newExpanded);
	};

	const filteredProviders = useMemo(() => {
		if (!searchQuery.trim()) return providers;

		const query = searchQuery.toLowerCase();
		return providers.filter((provider) => {
			const matchesProviderName = provider.displayName.toLowerCase().includes(query);
			const matchesModelName = provider.supportedModels.some((model) =>
				model.displayName.toLowerCase().includes(query) ||
				model.id.toLowerCase().includes(query)
			);
			const matchesCustomModel = customModels[provider.providerId]?.some((model) =>
				model.displayName.toLowerCase().includes(query) ||
				model.model_id.toLowerCase().includes(query)
			);

			return matchesProviderName || matchesModelName || matchesCustomModel;
		});
	}, [providers, customModels, searchQuery]);

	const isModelSelected = (model: ModelMetadata, provider: string, isCustom: boolean) => {
		if (selectedProvider !== provider) return false;
		if (!selectedModel) return false;

		// Only custom models can be selected (static models are display-only)
		if (!isCustom || !selectedIsCustom) return false;

		// Check id (UUID) to match custom models
		const customModel = model as CustomModel;
		const selectedCustomModel = selectedModel as CustomModel;
		return customModel.id === selectedCustomModel.id;
	};

	return (
		<div className="w-80 border-r border-stone-200 dark:border-stone-800 bg-white dark:bg-stone-900 flex flex-col">
			{/* Search */}
			<div className="p-4 border-b border-stone-200 dark:border-stone-800">
				<div className="relative">
					<SearchIcon className="absolute left-3 top-1/2 transform -translate-y-1/2 h-4 w-4 text-stone-400" />
					<Input
						placeholder={getMessage().OPENGROUND_SEARCH_MODELS}
						value={searchQuery}
						onChange={(e) => setSearchQuery(e.target.value)}
						className="pl-9"
					/>
				</div>
			</div>

			{/* Provider List */}
			<div className="flex-1 overflow-y-auto">
				{loading ? (
					<div className="p-4 text-center text-sm text-stone-500">
						{getMessage().LOADING}...
					</div>
				) : filteredProviders.length === 0 ? (
					<div className="p-4 text-center text-sm text-stone-500">
						{getMessage().OPENGROUND_NO_MODELS_FOUND}
					</div>
				) : (
					<div className="p-2">
						{filteredProviders.map((provider) => {
							const isExpanded = expandedProviders.has(provider.providerId);
							const providerCustomModels = customModels[provider.providerId] || [];
							const totalModels = provider.supportedModels.length + providerCustomModels.length;

							return (
								<div key={provider.providerId} className="mb-2">
									{/* Provider Header */}
									<button
										onClick={() => toggleProvider(provider.providerId)}
										className="w-full flex items-center justify-between p-2 rounded-lg hover:bg-stone-100 dark:hover:bg-stone-800 transition-colors"
									>
										<div className="flex items-center gap-2">
											{isExpanded ? (
												<ChevronDownIcon className="h-4 w-4 text-stone-500" />
											) : (
												<ChevronRightIcon className="h-4 w-4 text-stone-500" />
											)}
											<span className="font-medium text-sm text-stone-900 dark:text-stone-100">
												{provider.displayName}
											</span>
											<Badge variant="secondary" className="text-xs">
												{totalModels}
											</Badge>
										</div>
										<Button
											variant="ghost"
											size="sm"
											className="h-6 w-6 p-0"
											onClick={(e) => {
												e.stopPropagation();
												onAddNew(provider.providerId);
											}}
										>
											<PlusIcon className="h-3 w-3" />
										</Button>
									</button>

									{/* Models List */}
									{isExpanded && (
										<div className="ml-4 mt-1 space-y-1">
											{/* Static Models - Display only, not editable */}
											{provider.supportedModels.map((model) => (
												<div
													key={model.id}
													className="w-full text-left p-2 rounded-md border-l-2 border-transparent opacity-75"
												>
													<div className="text-sm font-medium text-stone-700 dark:text-stone-300">{model.displayName}</div>
													<div className="flex items-center gap-2 mt-1">
														<Badge variant="outline" className="text-xs">
															{model.contextWindow.toLocaleString()} tokens
														</Badge>
														<Badge variant="outline" className="text-xs">
															${model.inputPricePerMToken}/M
														</Badge>
													</div>
												</div>
											))}

											{/* Custom Models Section */}
											{providerCustomModels.length > 0 && (
												<>
													<div className="text-xs font-medium text-stone-500 dark:text-stone-400 px-2 py-1 mt-2">
														{getMessage().OPENGROUND_CUSTOM_MODELS}
													</div>
													{providerCustomModels.map((model) => (
														<button
															key={model.id}
															onClick={() => onSelectModel(model, provider.providerId, true)}
															className={cn(
																"w-full text-left p-2 rounded-md transition-colors border-l-2",
																isModelSelected(model, provider.providerId, true)
																	? "bg-primary/10 dark:bg-primary/20 border-primary text-stone-900 dark:text-stone-100"
																	: "border-transparent hover:bg-stone-100 dark:hover:bg-stone-800 text-stone-700 dark:text-stone-300"
															)}
														>
															<div className="flex items-center gap-2">
																<div className="text-sm font-medium">{model.displayName}</div>
																<Badge className="text-xs h-4">Custom</Badge>
															</div>
															<div className="flex items-center gap-2 mt-1">
																<Badge variant="outline" className="text-xs">
																	{model.contextWindow.toLocaleString()} tokens
																</Badge>
																<Badge variant="outline" className="text-xs">
																	${model.inputPricePerMToken}/M
																</Badge>
															</div>
														</button>
													))}
												</>
											)}
										</div>
									)}
								</div>
							);
						})}
					</div>
				)}
			</div>
		</div>
	);
}
