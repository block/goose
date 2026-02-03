"use client";
import { lens } from "@dhmk/zustand-lens";
import { UserStore } from "@/types/store/user";

export const userStoreSlice: UserStore = lens((setStore, getStore) => ({
	details: undefined,
	isFetched: false,
	set: (u) =>
		setStore(() => ({
			details: u,
			isFetched: true,
		})),
	reset: () =>
		setStore(() => ({
			...getStore(),
			details: undefined,
			isFetched: false,
		})),
}));
