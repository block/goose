"use client";
import { DatabaseConfigStore } from "@/types/store/database-config";
import { lens } from "@dhmk/zustand-lens";


export const databaseConfigStoreSlice: DatabaseConfigStore = lens(
	(setStore, getStore) => ({
		ping: { status: "pending" },
		isLoading: false,
		setPing: ({ error, status }) =>
			setStore(() => ({
				...getStore(),
				ping: {
					error,
					status,
				},
			})),
		setList: (list) =>
			setStore(() => ({
				...getStore(),
				list,
				isLoading: false,
			})),
		setIsLoading: (isLoading?: boolean) =>
			setStore(() => ({
				...getStore(),
				isLoading: !!isLoading,
			})),
	})
);
