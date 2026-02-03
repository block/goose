import { DatabaseConfigWithActive } from "@/constants/dbConfig";
import { useRootStore } from "@/store";
import { deleteData, getData } from "@/utils/api";
import asaw from "@/utils/asaw";
import { toast } from "sonner";

export const fetchDatabaseConfigList = async (
	successCb: (data: any[]) => void
) => {
	useRootStore.getState().databaseConfig.setIsLoading(true);
	const [, data] = await asaw(
		getData({
			method: "GET",
			url: "/api/db-config",
		})
	);

	successCb(data || []);
	useRootStore.getState().databaseConfig.setList(data || []);
};

export const pingActiveDatabaseConfig = async () => {
	useRootStore.getState().databaseConfig.setPing({
		error: undefined,
		status: "pending",
	});
	const [err, data] = await asaw(
		getData({
			url: "/api/clickhouse",
			method: "POST",
		})
	);

	const error = err || data?.err;
	useRootStore.getState().databaseConfig.setPing({
		error: error,
		status: error ? "failure" : "success",
	});
};

export const changeActiveDatabaseConfig = async (
	databaseConfigId: string,
	successCb: () => void
) => {
	const [err, data] = await asaw(
		getData({
			method: "POST",
			url: `/api/db-config/current/${databaseConfigId}`,
		})
	);

	if (err || data?.err) {
		toast.error(err || data?.err || `Db config: setting active failed!`, {
			id: "db-config-current",
		});
		return;
	}

	successCb();
	const list = useRootStore.getState().databaseConfig.list || [];
	const updatedList = list.reduce((acc: DatabaseConfigWithActive[], item) => {
		if (item.id === databaseConfigId) acc.push({ ...item, isCurrent: true });
		else acc.push({ ...item, isCurrent: false });
		return acc;
	}, []);
	useRootStore.getState().databaseConfig.setList(updatedList);
	toast.success(`Db config: set active successfully!`, {
		id: "db-config-current",
	});
	pingActiveDatabaseConfig();
};

export const deleteDatabaseConfig = async (databaseConfigId: string) => {
	const [err, data] = await asaw(
		deleteData({
			url: `/api/db-config/${databaseConfigId}`,
		})
	);

	if (err || data?.err) {
		toast.error(err || data?.err || `Db config: deletion failed!`, {
			id: "db-config",
		});
		return;
	}

	const list = useRootStore.getState().databaseConfig.list || [];
	const updatedList = list.filter((item) => item.id !== databaseConfigId);
	useRootStore.getState().databaseConfig.setList(updatedList);
};
