"use client";
import { lens } from "@dhmk/zustand-lens";
import { concat, filter, set } from "lodash";
import {
	EvalutatedResponseData,
	OpengroundStore,
	Providers,
	PromptSource,
	SelectedProvider,
} from "@/types/store/openground";
import { ProviderResult } from "@/lib/platform/openground-clickhouse";

export const opengroundStoreSlice: OpengroundStore = lens(
	(setStore, getStore) => ({
		// Legacy fields
		selectedProviders: [],
		prompt: "",
		isLoading: false,

		// New fields
		promptSource: {
			type: "custom",
			content: "",
			variables: {},
		},
		selectedProvidersNew: [],
		evaluatedResponse: {
			isLoading: false,
		},
		availableProviders: [],
		setProviderConfig: (path: string, value: any) => {
			const storeValue = getStore();
			const selectedProviders = set(storeValue.selectedProviders, path, value);
			setStore(() => ({
				...storeValue,
				selectedProviders: [...selectedProviders],
			}));
		},
		addProvider: (
			provider: Providers,
			configObject: Record<string, any> = {}
		) => {
			const storeValue = getStore();
			setStore(() => ({
				...storeValue,
				selectedProviders: concat(storeValue.selectedProviders, {
					provider,
					config: configObject,
				}),
			}));
		},
		removeProvider: (index: number) => {
			const storeValue = getStore();

			setStore(() => ({
				...storeValue,
				selectedProviders: filter(
					storeValue.selectedProviders,
					(_, i) => i !== index
				),
			}));
		},
		setPrompt: (prompt: string) => {
			const storeValue = getStore();
			setStore(() => ({
				...storeValue,
				prompt,
			}));
		},
		reset: () =>
			setStore(() => ({
				...getStore(),
				selectedProviders: [],
				selectedProvidersNew: [],
				prompt: "",
				promptSource: {
					type: "custom",
					content: "",
					variables: {},
				},
				isLoading: false,
				evaluatedResponse: {
					isLoading: false,
				},
			})),
		setEvaluatedLoading: (l: boolean) => {
			setStore(() => ({
				...getStore(),
				evaluatedResponse: {
					isLoading: l,
				},
			}));
		},
		setEvaluatedData: (data: ProviderResult[]) => {
			setStore(() => ({
				...getStore(),
				evaluatedResponse: {
					isLoading: false,
					data,
				},
			}));
		},

		// New actions
		setPromptSource: (source: PromptSource) => {
			setStore(() => ({
				...getStore(),
				promptSource: source,
			}));
		},
		setPromptVariable: (key: string, value: string) => {
			const storeValue = getStore();
			setStore(() => ({
				...storeValue,
				promptSource: {
					...storeValue.promptSource,
					variables: {
						...storeValue.promptSource.variables,
						[key]: value,
					},
				},
			}));
		},
		loadAvailableProviders: async () => {
			try {
				const response = await fetch("/api/openground/providers");
				if (response.ok) {
					const providers = await response.json();
					setStore(() => ({
						...getStore(),
						availableProviders: providers,
					}));
				}
			} catch (error) {
				console.error("Failed to load providers:", error);
			}
		},
		addProviderNew: (provider: string, model: string, hasVaultConfig: boolean) => {
			const storeValue = getStore();
			setStore(() => ({
				...storeValue,
				selectedProvidersNew: concat(storeValue.selectedProvidersNew, {
					provider,
					model,
					hasVaultConfig,
					config: {},
				}),
			}));
		},
		removeProviderNew: (index: number) => {
			const storeValue = getStore();
			setStore(() => ({
				...storeValue,
				selectedProvidersNew: filter(
					storeValue.selectedProvidersNew,
					(_, i) => i !== index
				),
			}));
		},
		setProviderConfigNew: (index: number, config: Record<string, any>) => {
			const storeValue = getStore();
			const providers = [...storeValue.selectedProvidersNew];
			if (providers[index]) {
				providers[index] = {
					...providers[index],
					config: {
						...providers[index].config,
						...config,
					},
				};
				setStore(() => ({
					...storeValue,
					selectedProvidersNew: providers,
				}));
			}
		},
		updateProviderModel: (index: number, model: string) => {
			const storeValue = getStore();
			const providers = [...storeValue.selectedProvidersNew];
			if (providers[index]) {
				providers[index] = {
					...providers[index],
					model,
				};
				setStore(() => ({
					...storeValue,
					selectedProvidersNew: providers,
				}));
			}
		},
	})
);
