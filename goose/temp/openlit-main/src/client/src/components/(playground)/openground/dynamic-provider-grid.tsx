"use client";

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { useRootStore } from "@/store";
import { CheckCircle2Icon, SettingsIcon } from "lucide-react";
import { useEffect, useState } from "react";
import ProviderConfigDialog from "./provider-config-dialog";
import useFetchWrapper from "@/utils/hooks/useFetchWrapper";
import { toast } from "sonner";
import getMessage from "@/constants/messages";

export default function DynamicProviderGrid() {
	const availableProviders = useRootStore((state) => state.openground.availableProviders);
	const selectedProvidersNew = useRootStore((state) => state.openground.selectedProvidersNew);
	const loadAvailableProviders = useRootStore((state) => state.openground.loadAvailableProviders);
	const addProviderNew = useRootStore((state) => state.openground.addProviderNew);

	const { data: providerConfigsData, fireRequest: fireConfigRequest } =
		useFetchWrapper<any[]>();
	const [selectedProvider, setSelectedProvider] = useState<string | null>(null);
	const [configDialogOpen, setConfigDialogOpen] = useState(false);
	const [providerConfigs, setProviderConfigs] = useState<Record<string, boolean>>({});

	useEffect(() => {
		loadAvailableProviders();
		loadProviderConfigs();
	}, []);

	useEffect(() => {
		if (providerConfigsData) {
			const configMap: Record<string, boolean> = {};
			providerConfigsData.forEach((config: any) => {
				configMap[config.provider] = config.isActive;
			});
			setProviderConfigs(configMap);
		}
	}, [providerConfigsData]);

	const loadProviderConfigs = () => {
		fireConfigRequest({
			requestType: "GET",
			url: "/api/openground/config",
			failureCb: (err?: string) => {
				toast.error(err || getMessage().OPENGROUND_LOAD_CONFIG_FAILED, {
					id: "provider-configs",
				});
			},
		});
	};

	const handleProviderClick = (providerId: string) => {
		const hasConfig = providerConfigs[providerId];

		if (hasConfig) {
			// Always allow adding provider (can select same provider multiple times with different models)
			const provider = availableProviders.find((p) => p.providerId === providerId);
			if (provider && provider.supportedModels.length > 0) {
				addProviderNew(providerId, provider.supportedModels[0].id, true);
				toast.success(`${provider.displayName} ${getMessage().OPENGROUND_PROVIDER_ADDED}`, {
					id: "provider-add",
				});
			}
		} else {
			// Open config dialog
			setSelectedProvider(providerId);
			setConfigDialogOpen(true);
		}
	};

	const handleEditConfig = (providerId: string, event: React.MouseEvent) => {
		event.stopPropagation();
		setSelectedProvider(providerId);
		setConfigDialogOpen(true);
	};

	const getProviderCount = (providerId: string) => {
		return selectedProvidersNew.filter((p) => p.provider === providerId).length;
	};

	const handleConfigComplete = () => {
		setConfigDialogOpen(false);
		setSelectedProvider(null);
		loadProviderConfigs();
	};

	return (
		<>
			<Card className="border-stone-200 dark:border-stone-800">
				<CardHeader>
					<CardTitle className="text-lg">{getMessage().OPENGROUND_SELECT_PROVIDERS}</CardTitle>
				</CardHeader>
				<CardContent>
					<div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-6 gap-3 overflow-auto">
						{availableProviders.map((provider) => {
							const hasConfig = providerConfigs[provider.providerId];
							const providerCount = getProviderCount(provider.providerId);

							return (
								<Card
									key={provider.providerId}
									className={`relative cursor-pointer transition-all hover:shadow-md ${providerCount > 0
										? "border-blue-500 dark:border-blue-400 bg-blue-50 dark:bg-blue-950/20"
										: hasConfig
											? "border-green-200 dark:border-green-800 hover:border-green-500"
											: "border-stone-200 dark:border-stone-700 hover:border-blue-400"
										}`}
									onClick={() => handleProviderClick(provider.providerId)}
								>
									{providerCount > 0 && (
										<Badge
											className="absolute -top-2 -right-2 h-6 w-6 rounded-full p-0 flex items-center justify-center z-10 bg-blue-600 hover:bg-blue-700"
										>
											{providerCount}
										</Badge>
									)}
									{hasConfig && (
										<Button
											variant="ghost"
											size="sm"
											className="absolute top-2 right-2 h-6 w-6 p-0 hover:bg-stone-200 dark:hover:bg-stone-700 z-10"
											onClick={(e) => handleEditConfig(provider.providerId, e)}
										>
											<SettingsIcon className="h-3 w-3" />
										</Button>
									)}
									<CardContent className="p-4 text-center">
										<div className="flex flex-col items-center gap-2">
											<div
												className={`w-12 h-12 rounded-lg flex items-center justify-center text-2xl font-bold ${providerCount > 0
													? "bg-blue-500 text-white"
													: "bg-stone-100 dark:bg-stone-800 text-stone-600 dark:text-stone-400"
													}`}
											>
												{provider.displayName.charAt(0)}
											</div>
											<div className="space-y-1">
												<p className="font-medium text-sm text-stone-900 dark:text-stone-100">
													{provider.displayName}
												</p>
												<p className="text-xs text-stone-500 dark:text-stone-400">
													{provider.supportedModels.length} models
												</p>
											</div>
											{hasConfig ? (
												<Badge
													variant="outline"
													className="text-xs bg-green-50 text-green-700 dark:bg-green-950/20 dark:text-green-300 border-green-200 dark:border-green-800"
												>
													<CheckCircle2Icon className="h-3 w-3 mr-1" />
													{getMessage().CONFIGURED}
												</Badge>
											) : (
												<Badge
													variant="outline"
													className="text-xs bg-yellow-50 text-yellow-700 dark:bg-yellow-950/20 dark:text-yellow-300 border-yellow-200 dark:border-yellow-800"
												>
													<SettingsIcon className="h-3 w-3 mr-1" />
													{getMessage().CONFIGURE}
												</Badge>
											)}
										</div>
									</CardContent>
								</Card>
							);
						})}
					</div>

					{selectedProvidersNew.length > 0 && (
						<div className="mt-4 p-4 bg-blue-50 dark:bg-blue-950/20 rounded-lg border border-blue-200 dark:border-blue-800">
							<p className="text-sm text-blue-900 dark:text-blue-100 font-medium">
								{selectedProvidersNew.length} {getMessage().PROVIDERS} {getMessage().SELECTED}
							</p>
							<p className="text-xs text-blue-700 dark:text-blue-300 mt-1">
								{getMessage().OPENGROUND_CLICK_PROVIDER_CARD_TO_CHANGE_MODEL_OR_SETTINGS}
							</p>
						</div>
					)}
				</CardContent>
			</Card>

			{selectedProvider && (
				<ProviderConfigDialog
					providerId={selectedProvider}
					open={configDialogOpen}
					onOpenChange={setConfigDialogOpen}
					onComplete={handleConfigComplete}
				/>
			)}
		</>
	);
}
