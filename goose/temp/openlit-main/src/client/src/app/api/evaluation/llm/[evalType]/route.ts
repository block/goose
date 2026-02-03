import { MetricParams, TimeLimit } from "@/lib/platform/common";
import { getEvaluationDetectedByType } from "@/lib/platform/evaluation";
import {
	validateMetricsRequest,
	validateMetricsRequestType,
} from "@/helpers/server/platform";
import { NextRequest } from "next/server";

export async function POST(
	request: NextRequest,
	{ params: { evalType } }: { params: { evalType: string } }
) {
	const formData = await request.json();
	const timeLimit = formData.timeLimit as TimeLimit;

	const params: MetricParams = {
		timeLimit,
	};

	const validationParam = validateMetricsRequest(
		params,
		validateMetricsRequestType.GET_TOTAL_EVALUATION_DETECTED
	);

	if (!validationParam.success)
		return Response.json(validationParam.err, {
			status: 400,
		});

	const res: any = await getEvaluationDetectedByType(params, evalType);
	return Response.json(res);
}
