"use client";

import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select";
import { useRootStore } from "@/store";
import { useState, useEffect } from "react";
import { toast } from "sonner";
import { KeyIcon, CheckCircle2Icon } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import useFetchWrapper from "@/utils/hooks/useFetchWrapper";
import SecretForm from "@/components/(playground)/vault/form";
import getMessage from "@/constants/messages";

interface ProviderConfigDialogProps {
	providerId: string;
	open: boolean;
	onOpenChange: (open: boolean) => void;
	onComplete: () => void;
}

interface VaultSecret {
	id: string;
	key: string;
	createdAt: string;
}

export default function ProviderConfigDialog({
	providerId,
	open,
	onOpenChange,
	onComplete,
}: ProviderConfigDialogProps) {
	const availableProviders = useRootStore((state) => state.openground.availableProviders);
	const { data: vaultSecrets, fireRequest: fireVaultRequest, isLoading: loadingSecrets } =
		useFetchWrapper<VaultSecret[]>();
	const { data: existingConfig, fireRequest: fireConfigRequest, isLoading: loadingConfig } =
		useFetchWrapper<any>();
	const { fireRequest: fireSaveRequest, isLoading: loading } =
		useFetchWrapper();
	const [selectedVaultId, setSelectedVaultId] = useState<string>("");
	const [selectedModelId, setSelectedModelId] = useState<string>("");

	const provider = availableProviders.find((p) => p.providerId === providerId);

	useEffect(() => {
		if (open) {
			loadVaultSecrets();
			loadExistingConfig();
		}
	}, [open]);

	useEffect(() => {
		if (existingConfig && Array.isArray(existingConfig) && existingConfig.length > 0) {
			const config = existingConfig.find((c: any) => c.provider === providerId);
			if (config) {
				setSelectedVaultId(config.vaultId || "");
				setSelectedModelId(config.modelId || "");
			}
		}
	}, [existingConfig, providerId]);

	const loadVaultSecrets = () => {
		fireVaultRequest({
			requestType: "POST",
			url: "/api/vault/get",
			body: JSON.stringify(""),
			failureCb: (err?: string) => {
				toast.error(err || getMessage().OPENGROUND_LOAD_VAULT_KEYS_FAILED, {
					id: "vault-secrets",
				});
			},
		});
	};

	const loadExistingConfig = () => {
		fireConfigRequest({
			requestType: "GET",
			url: "/api/openground/config",
			failureCb: (err?: string) => {
				// Silently fail if no config exists yet
				console.log("No existing config:", err);
			},
		});
	};

	const handleSave = () => {
		if (!selectedVaultId) {
			toast.error(getMessage().OPENGROUND_SELECT_API_KEY_ERROR);
			return;
		}

		const isUpdating = existingConfig && Array.isArray(existingConfig) &&
			existingConfig.some((c: any) => c.provider === providerId);

		fireSaveRequest({
			requestType: "POST",
			url: "/api/openground/config",
			body: JSON.stringify({
				provider: providerId,
				vaultId: selectedVaultId,
				modelId: selectedModelId || undefined,
				isActive: true,
			}),
			successCb: (data: any) => {
				toast.success(
					`${provider?.displayName} ${isUpdating ? getMessage().OPENGROUND_CONFIG_UPDATED : getMessage().OPENGROUND_CONFIG_SAVED}`,
					{
						id: "provider-config",
					}
				);
				onComplete();
			},
			failureCb: (err?: string) => {
				toast.error(err || getMessage().OPENGROUND_SAVE_CONFIG_FAILED, {
					id: "provider-config",
				});
			},
		});
	};

	if (!provider) return null;

	const isUpdating = existingConfig && Array.isArray(existingConfig) &&
		existingConfig.some((c: any) => c.provider === providerId);

	return (
		<Dialog open={open} onOpenChange={onOpenChange}>
			<DialogContent className="sm:max-w-[500px]">
				<DialogHeader>
					<DialogTitle className="flex items-center gap-2">
						<div className="w-10 h-10 rounded-lg bg-stone-100 dark:bg-stone-800 flex items-center justify-center text-lg font-bold text-stone-600 dark:text-stone-400">
							{provider.displayName.charAt(0)}
						</div>
						{isUpdating ? getMessage().OPENGROUND_UPDATE : getMessage().CONFIGURE} {provider.displayName}
					</DialogTitle>
					<DialogDescription>
						{getMessage().OPENGROUND_LINK_PROVIDER_TO_VAULT_DESCRIPTION}
					</DialogDescription>
				</DialogHeader>

				<div className="space-y-4 py-4">
					{/* Vault Secret Selection */}
					<div className="space-y-2">
						<Label htmlFor="vault-secret" className="flex items-center gap-2">
							<KeyIcon className="h-4 w-4" />
							{getMessage().OPENGROUND_API_KEY_FROM_VAULT}
						</Label>
						<Select
							value={selectedVaultId}
							onValueChange={setSelectedVaultId}
							disabled={loadingSecrets}
						>
							<SelectTrigger id="vault-secret">
								<SelectValue
									placeholder={
										loadingSecrets
											? getMessage().OPENGROUND_LOADING_SECRETS
											: getMessage().OPENGROUND_SELECT_API_KEY
									}
								/>
							</SelectTrigger>
							<SelectContent>
								{(!vaultSecrets || vaultSecrets.length === 0) && !loadingSecrets ? (
									<div className="p-3 flex flex-col gap-2">
										<p className="text-sm text-stone-600 dark:text-stone-400">
											{getMessage().OPENGROUND_NO_API_KEYS_FOUND_IN_VAULT}
										</p>
										<SecretForm successCallback={loadVaultSecrets}>
											<Button
												variant="outline"
												size="sm"
												className="w-full text-primary hover:text-primary"
											>
												<KeyIcon className="h-4 w-4 mr-2" />
												{getMessage().OPENGROUND_CREATE_NEW_API_KEY}
											</Button>
										</SecretForm>
									</div>
								) : (
									vaultSecrets?.map((secret) => (
										<SelectItem key={secret.id} value={secret.id}>
											<div className="flex items-center gap-2">
												<KeyIcon className="h-3 w-3" />
												{secret.key}
											</div>
										</SelectItem>
									))
								)}
							</SelectContent>
						</Select>
						<p className="text-xs text-stone-500 dark:text-stone-400">
							{provider.displayName} {getMessage().OPENGROUND_API_KEY_STORED_IN_VAULT}
						</p>
					</div>

					{/* Model Selection */}
					<div className="space-y-2">
						<Label htmlFor="model">{getMessage().OPENGROUND_DEFAULT_MODEL_OPTIONAL}</Label>
						<Select value={selectedModelId} onValueChange={setSelectedModelId}>
							<SelectTrigger id="model" className="h-auto">
								<SelectValue placeholder={getMessage().OPENGROUND_SELECT_DEFAULT_MODEL} />
							</SelectTrigger>
							<SelectContent>
								{provider.supportedModels.map((model) => (
									<SelectItem key={model.id} value={model.id}>
										<div className="flex flex-col items-start gap-1">
											<span>{model.displayName}</span>
											<div className="flex gap-2 text-xs">
												<Badge variant="secondary" className="text-xs">
													{model.contextWindow.toLocaleString()} tokens
												</Badge>
												<Badge variant="outline" className="text-xs">
													${model.inputPricePerMToken}/M in
												</Badge>
												<Badge variant="outline" className="text-xs">
													${model.outputPricePerMToken}/M out
												</Badge>
											</div>
										</div>
									</SelectItem>
								))}
							</SelectContent>
						</Select>
						<p className="text-xs text-stone-500 dark:text-stone-400">
							{getMessage().OPENGROUND_YOU_CAN_CHANGE_PER_EVALUATION}
						</p>
					</div>

					{/* Info Box */}
					<div className="p-3 bg-blue-50 dark:bg-blue-950/20 rounded-lg border border-blue-200 dark:border-blue-800">
						<div className="flex gap-2">
							<CheckCircle2Icon className="h-4 w-4 text-blue-600 dark:text-blue-400 mt-0.5" />
							<div className="text-sm text-blue-900 dark:text-blue-100">
								<p className="font-medium">{getMessage().CONFIGURATION_STORED_SECURELY}</p>
								<p className="text-xs text-blue-700 dark:text-blue-300 mt-1">
									{getMessage().OPENGROUND_API_KEY_REFERENCE_SAVED_INFO}
								</p>
							</div>
						</div>
					</div>
				</div>

				<DialogFooter>
					<Button variant="outline" onClick={() => onOpenChange(false)}>
						{getMessage().CANCEL}
					</Button>
					<Button onClick={handleSave} disabled={loading || loadingConfig || !selectedVaultId}>
						{loading ? (isUpdating ? getMessage().UPDATING : getMessage().SAVING) : (isUpdating ? getMessage().OPENGROUND_UPDATE_CONFIGURATION : getMessage().OPENGROUND_SAVE_CONFIGURATION)}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}
