import { useRootStore } from "@/store";
import { RootStore } from "@/types/store/root";

export const getFilterDetails = (state: RootStore) => state.filter.details;

export const getUpdateFilter = (state: RootStore) => state.filter.updateFilter;

export const getFilterConfig = (state: RootStore) => state.filter.config;

export const getUpdateConfig = (state: RootStore) => state.filter.updateConfig;

export const useFilters = () => {
	return useRootStore((state: RootStore) => state.filter);
};
