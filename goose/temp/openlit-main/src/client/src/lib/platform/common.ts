import { Pool } from "generic-pool";
import { getDBConfigById, getDBConfigByUser } from "../db-config";
import createClickhousePool from "./clickhouse/clickhouse-client";
import asaw from "@/utils/asaw";
import {
	ClickHouseClient,
	QueryParams,
	InsertParams,
	ExecParams,
	CommandParams,
} from "@clickhouse/client-common";
import { OPERATION_TYPE } from "@/types/platform";

export const OTEL_TRACES_TABLE_NAME = "otel_traces";
export const OTEL_GPUS_TABLE_NAME = "otel_metrics_gauge";

export type TimeLimit = {
	start: Date | string;
	end: Date | string;
	type: string;
};

export interface MetricParams {
	timeLimit: TimeLimit;
	offset?: number;
	limit?: number;
	selectedConfig?: any;
	sorting?: any;
	operationType?: OPERATION_TYPE;
	statusCode?: string[];
}

export type GPU_TYPE_KEY =
	| "utilization"
	| "enc.utilization"
	| "dec.utilization"
	| "temperature"
	| "fan_speed"
	| "memory.available"
	| "memory.total"
	| "memory.used"
	| "memory.free"
	| "power.draw"
	| "power.limit";

export interface GPUMetricParams extends MetricParams { }

export type DataCollectorType = { err?: unknown; data?: unknown };
export async function dataCollector(
	{
		query,
		format = "JSONEachRow",
		table,
		values,
		enable_readonly = false,
	}: Partial<QueryParams & InsertParams & ExecParams & CommandParams & { enable_readonly?: boolean }>,
	clientQueryType: "query" | "command" | "insert" | "exec" | "ping" = "query",
	dbConfigId?: string
): Promise<DataCollectorType> {
	let err, dbConfig;
	if (dbConfigId) {
		[err, dbConfig] = await asaw(getDBConfigById({ id: dbConfigId }));
	} else {
		[err, dbConfig] = await asaw(getDBConfigByUser(true));
	}

	if (err) return { err, data: [] };
	let clickhousePool: Pool<ClickHouseClient> | undefined;
	let client: ClickHouseClient | undefined;

	try {
		clickhousePool = createClickhousePool(dbConfig);
		const [err, clientClick] = await asaw(clickhousePool.acquire());

		if (err) {
			return { err, data: [] };
		}
		client = clientClick;
		if (!client)
			return { err: "Clickhouse client is not available!", data: [] };
		let respErr;
		let result;

		if (clientQueryType === "query") {
			if (!query) return { err: "No query specified!" };
			const object: QueryParams = {
				query,
				format,
			}
			if (enable_readonly) {
				object.clickhouse_settings = {
					readonly: "2",
				}
			}

			[respErr, result] = await asaw(
				client.query(object)
			);

			if (result) {
				const [err, data] = await asaw(result?.json());
				return { err, data };
			}
		} else if (clientQueryType === "insert") {
			if (!table || !values) return { err: "No table specified!" };
			[respErr, result] = await asaw(
				client.insert({
					table,
					values,
					format,
				})
			);

			if (!respErr) {
				return { data: result };
			}
		} else if (clientQueryType === "exec") {
			if (!query) return { err: "No query specified!" };
			[respErr, result] = await asaw(
				client.exec({
					query,
				})
			);

			if (!respErr) {
				return { data: result };
			}
		} else if (clientQueryType === "ping") {
			[respErr, result] = await asaw(client.query({
				query: "SELECT 1",
			}));

			return { err: respErr, data: !!result };
		} else if (clientQueryType === "command") {
			if (!query) return { err: "No query specified!" };
			[respErr, result] = await asaw(
				client.command({
					query,
				})
			);

			if (result?.query_id) {
				return { data: "Query executed successfully!" };
			}
		}

		return { err: respErr || "Unable to process the information" };
	} catch (error: any) {
		return { err: `ClickHouse Query Error: ${error.message}`, data: [] };
	} finally {
		if (clickhousePool && client) clickhousePool?.release(client);
	}
}
