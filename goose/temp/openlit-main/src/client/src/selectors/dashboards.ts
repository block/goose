import { useRootStore } from "@/store";

export const useDashboardPageSearch = () => {
	const store = useRootStore();
	return store.dashboards.page.search;
}
export const useSetDashboardPageSearch = () => {
	const store = useRootStore();
	return store.dashboards.setPageSearch;
}