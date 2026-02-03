"use client";
import { lens } from "@dhmk/zustand-lens";
import { cloneDeep, set } from "lodash";
import { PageStore, DASHBOARD_TYPE_OBJECT } from "@/types/store/page";

export const pageStoreSlice: PageStore = lens((setStore, getStore) => ({
	dashboard: {
		type: DASHBOARD_TYPE_OBJECT.llm,
	},
	request: {
		visibilityColumns: {
			id: true,
			time: true,
			requestDuration: true,
			spanName: true,
			serviceName: true,
		},
	},
	exception: {
		visibilityColumns: {
			id: true,
			time: true,
			spanName: true,
			deploymentType: true,
			exceptionType: true,
		},
	},
	fleethub: {
		visibilityColumns: {
			id: true,
			name: true,
			os: true,
			version: true,
			startedAt: true,
			status: true,
		}
	},
	header: {
		title: "",
		breadcrumbs: [],
	},
	setHeader: (header) => {
		setStore({
			...getStore(),
			header: {
				...getStore().header,
				title: header.title,
				breadcrumbs: header.breadcrumbs,
				description: header.description,
			}
		});
	},
	setData: (page, keyPath, value) => {
		const store = getStore();
		let pageObject = cloneDeep(store[page as keyof typeof store]);

		set(pageObject, keyPath, value);

		setStore({
			...store,
			[page]: {
				...pageObject,
			},
		});
	},
}));
