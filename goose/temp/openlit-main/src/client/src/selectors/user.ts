import { RootStore } from "@/types/store/root";

export const getUserDetails = (state: RootStore) => state.user.details;

export const getIsUserFetched = (state: RootStore) => state.user.isFetched;

export const setUser = (state: RootStore) => state.user.set;

export const resetUser = (state: RootStore) => state.user.reset;
