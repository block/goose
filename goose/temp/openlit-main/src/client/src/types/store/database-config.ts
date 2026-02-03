import { DatabaseConfigWithActive } from "@/constants/dbConfig";

export type DatabaseConfigStorePingStatus = "success" | "failure" | "pending";

export type DatabaseConfigStore = {
	ping: {
		error?: string;
		status: DatabaseConfigStorePingStatus;
	};
	list?: DatabaseConfigWithActive[];
	isLoading: boolean;
	setPing: (obj: {
		error?: string;
		status: DatabaseConfigStorePingStatus;
	}) => void;
	setList: (u: DatabaseConfigWithActive[]) => void;
	setIsLoading: (f?: boolean) => void;
};
