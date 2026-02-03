import getMessage from "@/constants/messages";
import { getCurrentUser } from "@/lib/session";
import { throwIfError } from "@/utils/error";
import Sanitizer from "@/utils/sanitizer";
import { dataCollector, MetricParams, OTEL_TRACES_TABLE_NAME } from "../common";
import { OPENLIT_EVALUATION_TABLE_NAME } from "./table-details";
import { spawn } from "child_process";
import { getEvaluationConfig, getEvaluationConfigById } from "./config";
import asaw from "@/utils/asaw";
import {
	AutoEvaluationConfig,
	Evaluation,
	EvaluationConfig,
	EvaluationConfigWithSecret,
	EvaluationResponse,
} from "@/types/evaluation";
import { consoleLog } from "@/utils/log";
import { getRequestViaSpanId } from "../request";
import { getTraceMappingKeyFullPath } from "@/helpers/server/trace";
import { TraceRow } from "@/types/trace";
import { get } from "lodash";
import { getDBConfigById } from "@/lib/db-config";
import { SUPPORTED_EVALUATION_OPERATIONS } from "@/constants/traces";
import { jsonParse } from "@/utils/json";
import {
	getLastRunCronLogByCronId,
	getLastFailureCronLogBySpanId,
	insertCronLog,
} from "@/lib/platform/cron-log";
import { CronType, CronRunStatus, CronLogData } from "@/types/cron";
import { differenceInSeconds } from "date-fns";
import { getFilterPreviousParams } from "@/helpers/server/platform";

export async function getEvaluationsForSpanId(spanId: string) {
	const user = await getCurrentUser();

	throwIfError(!user, getMessage().UNAUTHORIZED_USER);
	const sanitizedSpanId = Sanitizer.sanitizeValue(spanId);

	const query = `
		SELECT 
			span_id as spanId,
			created_at as createdAt,
			id,
			arrayMap(
					(e, c, ex, v) -> 
					map('evaluation', e, 'score', if(mapContains(scores, e), toString(scores[e]), toString(0.0)), 'classification', c, 'explanation', ex, 'verdict', v),
					evaluationData.evaluation,
					evaluationData.classification,
					evaluationData.explanation,
					evaluationData.verdict
			) AS evaluations
		FROM ${OPENLIT_EVALUATION_TABLE_NAME}
		WHERE spanId = '${sanitizedSpanId}'
		ORDER BY created_at;
	`;

	const { data, err } = await dataCollector({ query });

	if (err) {
		return { err };
	}

	if (!(data as any[])?.length) {
		const [evaluationConfigErr, evaluationConfig] = await asaw(
			getEvaluationConfig()
		);
		const evaluationConfigTyped = evaluationConfig as EvaluationConfig;
		if (evaluationConfigErr) {
			return { configErr: evaluationConfigErr };
		}
		if (!evaluationConfigTyped?.id) {
			return { configErr: getMessage().EVALUATION_CONFIG_NOT_FOUND };
		}

		const { data: lastFailureCronLog } = await getLastFailureCronLogBySpanId(
			spanId
		);

		const errorMessage =
			(lastFailureCronLog as CronLogData[])?.[0]?.errorStacktrace?.[spanId] ||
			"";

		if (errorMessage) {
			return { err: errorMessage };
		}

		return { config: evaluationConfigTyped.id };
	}
	return { data: (data as EvaluationResponse[])?.[0] || {} };
}

async function storeEvaluation(
	spanId: string,
	evaluation: Evaluation[],
	meta: any,
	dbConfigId?: string
) {
	const { err } = await dataCollector(
		{
			table: OPENLIT_EVALUATION_TABLE_NAME,
			values: [
				{
					span_id: spanId,
					meta,
					"evaluationData.evaluation": evaluation.map((e) => e.evaluation),
					"evaluationData.classification": evaluation.map(
						(e) => e.classification
					),
					"evaluationData.explanation": evaluation.map((e) => e.explanation),
					"evaluationData.verdict": evaluation.map((e) => e.verdict),
					scores: evaluation.reduce((acc: Record<string, number>, e) => {
						acc[e.evaluation] = e.score;
						return acc;
					}, {}),
				},
			],
		},
		"insert",
		dbConfigId
	);
	if (err) {
		consoleLog(err);
		return { err };
	}

	return { data: true };
}

export async function setEvaluationsForSpanId(spanId: string) {
	const user = await getCurrentUser();

	throwIfError(!user, getMessage().UNAUTHORIZED_USER);
	const sanitizedSpanId = Sanitizer.sanitizeValue(spanId);

	let { record: spanData } = await getRequestViaSpanId(sanitizedSpanId);
	const spanDataTyped = spanData as TraceRow;

	throwIfError(!(spanData as any).SpanId, getMessage().TRACE_NOT_FOUND);

	const evaluationConfig = await getEvaluationConfig(undefined, false);
	return await getEvaluationConfigForTrace(spanDataTyped, evaluationConfig);
}

async function getEvaluationConfigForTrace(
	trace: TraceRow,
	evaluationConfig: EvaluationConfigWithSecret,
	dbConfigId?: string
) {
	const response = get(trace, getTraceMappingKeyFullPath("response", true));
	const prompt = get(trace, getTraceMappingKeyFullPath("prompt", true));
	// const contexts = spanDataTyped[getTraceMappingKeyFullPath("prompt", true)];

	try {
		const data: { success: boolean; result?: Evaluation[]; error?: string } =
			await new Promise((resolve) => {
				const pythonProcess = spawn("/bin/bash", [
					"-c",
					`
					. venv/bin/activate && \
					python3 scripts/evaluation/evaluate.py '${JSON.stringify({
						spanId: trace.SpanId,
						model: `${evaluationConfig.provider}/${evaluationConfig.model}`,
						api_key: evaluationConfig.secret.value,
						prompt,
						response,
						contexts: "",
						threshold_score: 0.5,
					})}' && \
					deactivate
				`,
				]);

				pythonProcess.on("error", (err) => {
					resolve({
						success: false,
						error: `Python process error: ${err.message}`,
					});
				});

				let output = "";
				let errorOutput = "";

				pythonProcess.stdout.on("data", (data) => {
					output += data.toString();
				});

				pythonProcess.stderr.on("data", (data) => {
					errorOutput += data.toString();
				});

				pythonProcess.on("close", (code) => {
					if (code === 0) {
						const match = output.match(/\{.*\}/m);
						if (match) {
							const parsedData = jsonParse(match[0]);
							return resolve(parsedData);
						}

						return resolve({ success: false, error: output });
					} else {
						resolve({ success: false, error: errorOutput });
					}
				});
			});

		if (!data.success) {
			return { success: false, error: data.error };
		}

		await storeEvaluation(
			trace.SpanId,
			data.result || [],
			{
				model: `${evaluationConfig.provider}/${evaluationConfig.model}`,
				traceTimeStamp: trace.Timestamp,
			},
			dbConfigId
		);
		return { success: true };
	} catch (e) {
		consoleLog(e);
		return { success: false, error: e };
	}
}

export async function autoEvaluate(autoEvaluationConfig: AutoEvaluationConfig) {
	const startedAt = new Date();
	const cronLogObject = {
		cronId: autoEvaluationConfig.cronId,
		cronType: CronType.SPAN_EVALUATION,
		metaProperties: {
			...autoEvaluationConfig,
		},
		startedAt,
	};

	// TODO: Verify the cron job request
	const [evaluationConfigErr, evaluationConfig] = await asaw(
		getEvaluationConfigById(autoEvaluationConfig.evaluationConfigId, false)
	);

	if (evaluationConfigErr || !evaluationConfig.id) {
		return { err: getMessage().EVALUATION_CONFIG_NOT_FOUND, success: false };
	}

	const [databaseConfigErr, databaseConfig] = await asaw(
		getDBConfigById({ id: evaluationConfig.databaseConfigId })
	);

	if (databaseConfigErr || !databaseConfig.id) {
		return {
			err: getMessage().DATABASE_CONFIG_NOT_FOUND,
			success: false,
		};
	}

	const lastRunTime = await getLastRunCronLogByCronId(
		autoEvaluationConfig.cronId
	);

	const keyPath = `SpanAttributes['${getTraceMappingKeyFullPath("type")}']`;

	const query = `
		SELECT 
				*
		FROM ${OTEL_TRACES_TABLE_NAME} AS t
		LEFT JOIN ${OPENLIT_EVALUATION_TABLE_NAME} AS e 
		ON t.SpanId = e.span_id
		WHERE ${keyPath} IN (${SUPPORTED_EVALUATION_OPERATIONS.map(
		(operation) => `'${operation}'`
	).join(", ")})
		AND (e.span_id = '' OR e.span_id IS NULL)
		${
			lastRunTime
				? `AND t.Timestamp >= parseDateTimeBestEffort('${lastRunTime}')`
				: ""
		}
		ORDER BY t.Timestamp;
	`;

	const { data, err } = await dataCollector(
		{ query },
		"query",
		databaseConfig.id
	);

	if (err) {
		const finishedAt = new Date();
		const { err: cronLogErr } = await insertCronLog(
			{
				...cronLogObject,
				runStatus: CronRunStatus.FAILURE,
				errorStacktrace: {
					error: `${getMessage().TRACE_FETCHING_ERROR} : ${err}`,
				},
				finishedAt,
				duration: differenceInSeconds(finishedAt, startedAt),
			},
			databaseConfig.id
		);
		return { err: cronLogErr || err, success: false };
	}

	const traces = data as TraceRow[];
	let errorCount = 0;

	const results = await Promise.all(
		traces.map(async (trace) => {
			return await getEvaluationConfigForTrace(
				trace,
				evaluationConfig,
				evaluationConfig.databaseConfigId
			);
		})
	);

	const errorObject = results.reduce(
		(acc: Record<string, string>, r, index) => {
			if (!r.success) {
				acc[traces[index].SpanId] = r.error as string;
				errorCount++;
			}
			return acc;
		},
		{}
	);

	const finishedAt = new Date();
	const { err: cronLogErr } = await insertCronLog(
		{
			...cronLogObject,
			runStatus:
				errorCount === results.length
					? CronRunStatus.FAILURE
					: errorCount > 0
					? CronRunStatus.PARTIAL_SUCCESS
					: CronRunStatus.SUCCESS,
			errorStacktrace: errorObject,
			meta: {
				totalSpans: traces.length,
				totalEvaluated: results.length - errorCount,
				totalFailed: errorCount,
				spanIds: traces.map((t) => t.SpanId),
			},
			finishedAt,
			duration: differenceInSeconds(finishedAt, startedAt),
		},
		databaseConfig.id
	);

	return { success: true, err: cronLogErr };
}

export async function getEvaluationDetectedByType(
	params: MetricParams,
	evalType: string
) {
	const currentWhereParams = params;
	const previousWhereParams = getFilterPreviousParams(currentWhereParams);

	const commonQuery = (parameters: MetricParams) => `
		SELECT
			COUNT(DISTINCT span_id) AS total_evaluation_detected,
			'${params.timeLimit.start}' as start_date
		FROM ${OPENLIT_EVALUATION_TABLE_NAME} 
		ARRAY JOIN evaluationData
		WHERE 
			evaluationData.evaluation = '${evalType}'
			AND evaluationData.verdict = 'yes'
			AND parseDateTimeBestEffort(meta['traceTimeStamp']) >= parseDateTimeBestEffort('${parameters.timeLimit.start}') 
			AND parseDateTimeBestEffort(meta['traceTimeStamp']) <= parseDateTimeBestEffort('${parameters.timeLimit.end}')
	`;

	const query = `
		SELECT
			CAST(current_data.total_evaluation_detected AS FLOAT) AS total_evaluation_detected,
			CAST(previous_day.total_evaluation_detected AS FLOAT) AS previous_total_evaluation_detected
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
