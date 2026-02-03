import { MetricParams, dataCollector, OTEL_TRACES_TABLE_NAME } from "../common";
import {
	getTraceMappingKeyFullPath,
	buildHierarchy,
} from "@/helpers/server/trace";
import {
	dateTruncGroupingLogic,
	getFilterPreviousParams,
	getFilterWhereCondition,
} from "@/helpers/server/platform";

export async function getRequestPerTime(params: MetricParams) {
	const { start, end } = params.timeLimit;
	const dateTrunc = dateTruncGroupingLogic(end as Date, start as Date);

	const query = `
		SELECT
			CAST(COUNT(*) AS INTEGER) AS total,
			formatDateTime(DATE_TRUNC('${dateTrunc}', Timestamp), '%Y/%m/%d %R') AS request_time
		FROM
			${OTEL_TRACES_TABLE_NAME}
		WHERE ${getFilterWhereCondition({ ...params, operationType: "llm" })}
		GROUP BY
			request_time
		ORDER BY
			request_time;
		`;

	return dataCollector({ query });
}

export async function getTotalRequests(params: MetricParams) {
	const commonQuery = (parameters: MetricParams) => `
		SELECT
			COUNT(*) AS total_requests,
			'${params.timeLimit.start}' as start_date
		FROM ${OTEL_TRACES_TABLE_NAME} 
		WHERE ${getFilterWhereCondition(parameters)}	
	`;

	const previousWhereParams = getFilterPreviousParams(params);
	const query = `
		SELECT
			CAST(current_data.total_requests AS INTEGER) AS total_requests,
			CAST(previous_day.total_requests AS INTEGER) AS previous_total_requests
		FROM
			(
				${commonQuery(params)}
			) as current_data
			JOIN
			(
				${commonQuery(previousWhereParams)}
			) as previous_day
		ON
			current_data.start_date = previous_day.start_date;
	`;

	return dataCollector({ query });
}

export async function getAverageRequestDuration(params: MetricParams) {
	const keyPath = `${getTraceMappingKeyFullPath("requestDuration")}`;

	const commonQuery = (parameters: MetricParams) => `
		SELECT
			AVG(${keyPath}) AS average_duration,
			'${params.timeLimit.start}' as start_date
		FROM ${OTEL_TRACES_TABLE_NAME}
		WHERE ${getFilterWhereCondition(parameters)} AND isFinite(${keyPath})
	`;

	const currentWhereParams = params;
	const previousWhereParams = getFilterPreviousParams(currentWhereParams);

	const query = `
		SELECT
			CAST(current_data.average_duration AS FLOAT) AS average_duration,
			CAST(previous_day.average_duration AS FLOAT) AS previous_average_duration
		FROM
			(
				${commonQuery(currentWhereParams)}
			) as current_data
			JOIN
			(
				${commonQuery(previousWhereParams)}
			) as previous_day
		ON
			current_data.start_date = previous_day.start_date;
	`;

	return dataCollector({ query });
}

export async function getRequestsConfig(params: MetricParams) {
	const select: string[] = [];

	select.push(
		`arrayConcat(
			arrayFilter(x -> x != '', groupArray(DISTINCT SpanAttributes['${getTraceMappingKeyFullPath(
			"provider"
		)}'])),
			arrayFilter(x -> x != '', groupArray(DISTINCT SpanAttributes['${getTraceMappingKeyFullPath(
			"system"
		)}']))
		) AS providers`
	);

	select.push(
		`CAST(MAX(toFloat64OrZero(SpanAttributes['${getTraceMappingKeyFullPath(
			"cost"
		)}'])) AS FLOAT) AS maxCost`
	);

	select.push(
		`arrayFilter(x -> x != '', ARRAY_AGG(DISTINCT SpanAttributes['${getTraceMappingKeyFullPath(
			"model"
		)}'])) AS models`
	);

	select.push(
		`arrayFilter(x -> x != '', ARRAY_AGG(DISTINCT SpanAttributes['${getTraceMappingKeyFullPath(
			"type"
		)}'])) AS traceTypes`
	);

	select.push(`CAST(COUNT(*) AS INTEGER) AS totalRows`);

	select.push(
		`arrayFilter(x -> x != '', ARRAY_AGG(DISTINCT ResourceAttributes['${getTraceMappingKeyFullPath(
			"applicationName"
		)}'])) AS applicationNames`
	);

	select.push(
		`arrayFilter(x -> x != '', ARRAY_AGG(DISTINCT ResourceAttributes['${getTraceMappingKeyFullPath(
			"environment"
		)}'])) AS environments`
	);

	const query = `SELECT ${select.join(", ")} FROM ${OTEL_TRACES_TABLE_NAME} 
			WHERE ${getFilterWhereCondition(params)}`;

	return dataCollector({ query });
}

export async function getRequests(params: MetricParams) {
	const { limit = 10, offset = 0 } = params;

	const countQuery = `SELECT CAST(COUNT(*) AS INTEGER) AS total	FROM ${OTEL_TRACES_TABLE_NAME} 
		WHERE ${getFilterWhereCondition(params, true)}`;

	const { data: dataTotal, err: errTotal } = await dataCollector({
		query: countQuery,
	});
	if (errTotal) {
		return {
			err: errTotal,
		};
	}

	const query = `SELECT *	FROM ${OTEL_TRACES_TABLE_NAME} 
		WHERE ${getFilterWhereCondition(params, true)}
		${params.sorting
			? params.sorting.type.includes("cost")
				? `ORDER BY toFloat64OrZero(${params.sorting.type}) ${params.sorting.direction} `
				: params.sorting.type.includes("tokens")
					? `ORDER BY toInt32OrZero(${params.sorting.type}) ${params.sorting.direction} `
					: `ORDER BY ${params.sorting.type} ${params.sorting.direction} `
			: `ORDER BY Timestamp desc `
		}
		LIMIT ${limit}
		OFFSET ${offset}`;

	const { data, err } = await dataCollector({ query });
	return {
		err,
		records: data,
		total: (dataTotal as any[])?.[0]?.total || 0,
	};
}

export async function getRequestViaSpanId(spanId: string) {
	const query = `SELECT *	FROM ${OTEL_TRACES_TABLE_NAME} 
		WHERE SpanId='${spanId}'`;

	const { data, err } = await dataCollector({ query });
	return {
		err,
		record: (data as unknown[])?.[0],
	};
}

export async function getRequestViaTraceId(traceId: string) {
	const query = `SELECT *	FROM ${OTEL_TRACES_TABLE_NAME} WHERE ${getTraceMappingKeyFullPath(
		"id"
	)}='${traceId}'`;

	const { data, err } = await dataCollector({ query });
	return {
		err,
		record: (data as unknown[])?.[0],
	};
}

export async function getHeirarchyViaSpanId(spanId: string) {
	const commonQuery = (
		dir: "upward" | "downward",
		id: string
	) => `WITH RECURSIVE trace_hierarchy AS (
						SELECT
								${getTraceMappingKeyFullPath("id")},
								${getTraceMappingKeyFullPath("parentSpanId")},
								${getTraceMappingKeyFullPath("spanId")},
								${getTraceMappingKeyFullPath("spanName")},
								${getTraceMappingKeyFullPath("requestDuration")},
								0 AS level
						FROM
								${OTEL_TRACES_TABLE_NAME}
						WHERE
								SpanId = '${id}' -- Starting SpanId
						
						UNION ALL

						SELECT
								ot.${getTraceMappingKeyFullPath("id")},
								ot.${getTraceMappingKeyFullPath("parentSpanId")},
								ot.${getTraceMappingKeyFullPath("spanId")},
								ot.${getTraceMappingKeyFullPath("spanName")},
								ot.${getTraceMappingKeyFullPath("requestDuration")},
								th.level + 1 AS level
						FROM
								${OTEL_TRACES_TABLE_NAME} ot
						INNER JOIN
								trace_hierarchy th
						ON
								${dir === "upward"
			? `ot.${getTraceMappingKeyFullPath(
				"spanId"
			)} = th.${getTraceMappingKeyFullPath("parentSpanId")}`
			: `ot.${getTraceMappingKeyFullPath(
				"parentSpanId"
			)} = th.${getTraceMappingKeyFullPath("spanId")}`
		}
				)
				SELECT *
				FROM trace_hierarchy
				ORDER BY level DESC;`;

	const { data: upwardData, err: upwardErr } = await dataCollector({
		query: commonQuery("upward", spanId),
	});

	if ((upwardData as any[])?.[0]?.SpanId) {
		const { data: downwardData, err: downwardErr } = await dataCollector({
			query: commonQuery("downward", (upwardData as any[])?.[0]?.SpanId),
		});

		const heirarchy = buildHierarchy(downwardData as any[]);

		return {
			err: upwardErr || downwardErr || (heirarchy ? null : "Error in fetching heirarchy"),
			record: heirarchy,
		};
	}

	return {
		err: "Error in fetching heirarchy",
		record: {},
	};
}

export async function getRequestExist() {
	const query = `SELECT COUNT(*) AS total_requests FROM ${OTEL_TRACES_TABLE_NAME}`;
	return dataCollector({ query });
}