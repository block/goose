import { OPENLIT_CRON_LOG_TABLE_NAME } from "./table-details";
import { dataCollector } from "../common";
import { CronLogData, CronRunStatus, GetCronLogsParams } from "@/types/cron";
import { format } from "date-fns";

export async function insertCronLog(
	data: CronLogData,
	databaseConfigId: string
) {
	return await dataCollector(
		{
			table: OPENLIT_CRON_LOG_TABLE_NAME,
			values: [
				{
					cron_id: data.cronId,
					cron_type: data.cronType,
					run_status: data.runStatus,
					meta: data.meta || {},
					error_stacktrace: data.errorStacktrace || {},
					started_at: format(data.startedAt, "yyyy-MM-dd HH:mm:ss"),
					finished_at: format(data.finishedAt, "yyyy-MM-dd HH:mm:ss"),
					duration: data.duration || 0,
				},
			],
		},
		"insert",
		databaseConfigId
	);
}

export async function getCronLogs({
	page = 1,
	limit = 10,
	cronId,
	cronType,
	runStatus,
	startDate,
	endDate,
}: GetCronLogsParams = {}) {
	const offset = (page - 1) * limit;

	const whereConditions = [];

	if (cronId) {
		whereConditions.push(`cron_id = '${cronId}'`);
	}
	if (cronType) {
		whereConditions.push(`cron_type = '${cronType}'`);
	}
	if (runStatus) {
		whereConditions.push(`run_status = '${runStatus}'`);
	}
	if (startDate) {
		whereConditions.push(
			`started_at >= parseDateTimeBestEffort('${startDate.toISOString()}')`
		);
	}
	if (endDate) {
		whereConditions.push(
			`started_at <= parseDateTimeBestEffort('${endDate.toISOString()}')`
		);
	}

	const whereClause =
		whereConditions.length > 0 ? `WHERE ${whereConditions.join(" AND ")}` : "";

	const countQuery = `
    SELECT count(*) as total 
    FROM ${OPENLIT_CRON_LOG_TABLE_NAME}
    ${whereClause}
  `;

	const { data: countResult, err: countErr } = await dataCollector(
		{ query: countQuery },
		"query"
	);

	if (countErr) {
		console.error("Error getting cron logs count:", countErr);
		throw countErr;
	}

	const logsQuery = `
    SELECT 
      cron_id as cronId,
      cron_type as cronType,
      run_status as runStatus,
      meta as meta,
      error_stacktrace as errorStacktrace,
      started_at as startedAt,
      finished_at as finishedAt,
      duration
    FROM ${OPENLIT_CRON_LOG_TABLE_NAME}
    ${whereClause}
    ORDER BY started_at DESC
    LIMIT ${limit}
    OFFSET ${offset}
  `;

	const { data: logs, err: logsErr } = await dataCollector(
		{ query: logsQuery },
		"query"
	);

	if (logsErr) {
		console.error("Error getting cron logs:", logsErr);
		throw logsErr;
	}

	return {
		data: logs,
		pagination: {
			total: Number((countResult as any[])[0].total),
			page,
			limit,
		},
	};
}

export async function getLastRunCronLogByCronId(cronId: string) {
	const query = `
		SELECT * FROM ${OPENLIT_CRON_LOG_TABLE_NAME} WHERE cron_id = '${cronId}' AND run_status = '${CronRunStatus.SUCCESS}'
		ORDER BY started_at DESC
		LIMIT 1
	`;

	const { data, err } = await dataCollector({ query }, "query");

	if (err || !data) {
		return null;
	}

	return (data as CronLogData[])[0].startedAt;
}

export async function getLastFailureCronLogBySpanId(spanId: string) {
	const query = `
		WITH last_failures AS (
    	SELECT error_stacktrace as errorStacktrace
			FROM ${OPENLIT_CRON_LOG_TABLE_NAME}
			WHERE run_status = '${CronRunStatus.FAILURE}' 
			AND has(JSONExtractArrayRaw(meta['spanIds']), '"${spanId}"')
			ORDER BY started_at DESC
			LIMIT 1
		)
			SELECT * 
			FROM last_failures
			WHERE NOT EXISTS (
				SELECT 1
				FROM ${OPENLIT_CRON_LOG_TABLE_NAME}
				WHERE run_status = '${CronRunStatus.SUCCESS}'
				AND has(JSONExtractArrayRaw(meta['spanIds']), '"${spanId}"')
			);
	`;

	return await dataCollector({ query }, "query");
}
