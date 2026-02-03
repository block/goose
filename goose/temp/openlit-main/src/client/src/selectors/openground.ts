import { RootStore } from "@/types/store/root";

export const getSelectedProviders = (state: RootStore) =>
	state.openground.selectedProviders;

export const getPrompt = (state: RootStore) => state.openground.prompt;

export const getIsLoading = (state: RootStore) => state.openground.isLoading;

export const setProviderConfig = (state: RootStore) =>
	state.openground.setProviderConfig;

export const addProvider = (state: RootStore) => state.openground.addProvider;

export const removeProvider = (state: RootStore) =>
	state.openground.removeProvider;

export const setPrompt = (state: RootStore) => state.openground.setPrompt;

export const resetOpenground = (state: RootStore) => state.openground.reset;

export const getEvaluatedResponse = (state: RootStore) => state.openground.evaluatedResponse;

export const setEvaluatedLoading = (state: RootStore) => state.openground.setEvaluatedLoading;

export const setEvaluatedData = (state: RootStore) => state.openground.setEvaluatedData;
