import { MetricParams, TimeLimit } from "@/lib/platform/common";
import { getAverageRequestDuration } from "@/lib/platform/request";
import {
	validateMetricsRequest,
	validateMetricsRequestType,
} from "@/helpers/server/platform";
import { OPERATION_TYPE } from "@/types/platform";

export async function POST(request: Request) {
	const formData = await request.json();
	const timeLimit = formData.timeLimit as TimeLimit;
	const operationType = formData.operationType as OPERATION_TYPE;

	const params: MetricParams = {
		timeLimit,
		operationType,
	};

	const validationParam = validateMetricsRequest(
		params,
		validateMetricsRequestType.AVERAGE_REQUEST_DURATION
	);

	if (!validationParam.success)
		return Response.json(validationParam.err, {
			status: 400,
		});

	const res: any = await getAverageRequestDuration(params);
	return Response.json(res);
}
