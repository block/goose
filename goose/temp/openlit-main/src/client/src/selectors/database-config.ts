import { RootStore } from "@/types/store/root";

export const getPingDetails = (state: RootStore) => state.databaseConfig.ping;

export const getPingStatus = (state: RootStore) => state.databaseConfig.ping.status;

export const getDatabaseConfigList = (state: RootStore) =>
	state.databaseConfig.list;

export const getDatabaseConfigListIsLoading = (state: RootStore) =>
	state.databaseConfig.isLoading;

export const setPing = (state: RootStore) => state.databaseConfig.setPing;

export const setDatabaseConfigList = (state: RootStore) =>
	state.databaseConfig.setList;

export const setDatabaseConfigListIsLoading = (state: RootStore) =>
	state.databaseConfig.setIsLoading;
