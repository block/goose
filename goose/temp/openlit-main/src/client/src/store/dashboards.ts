"use client";
import { DashboardStore } from "@/types/store/dashboards";
import { lens } from "@dhmk/zustand-lens";


export const dashboardStoreSlice: DashboardStore = lens(
	(setStore, getStore) => ({
		page: {
			search: "",
		},
		setPageSearch: (search) =>
			setStore(() => ({
				...getStore(),
				page: {
					...getStore().page,
					search,
				},
			})),
	})
);
