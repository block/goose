import { create } from "zustand";
import { withLenses } from "@dhmk/zustand-lens";
import { devtools } from "zustand/middleware";
import { userStoreSlice } from "./user";
import { filterStoreSlice } from "./filter";
import { databaseConfigStoreSlice } from "./database-config";
import { opengroundStoreSlice } from "./openground";
import { pageStoreSlice } from "./page";
import { RootStore } from "@/types/store/root";
import { dashboardStoreSlice } from "./dashboards";

export const useRootStore = create<RootStore>()(
	devtools(
		withLenses({
			user: userStoreSlice,
			filter: filterStoreSlice,
			databaseConfig: databaseConfigStoreSlice,
			openground: opengroundStoreSlice,
			page: pageStoreSlice,
			dashboards: dashboardStoreSlice,
		})
	)
);
