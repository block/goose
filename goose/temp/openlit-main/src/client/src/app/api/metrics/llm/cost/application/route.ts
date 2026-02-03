import { MetricParams, TimeLimit } from "@/lib/platform/common";
import { getCostByApplication } from "@/lib/platform/llm/cost";
import {
	validateMetricsRequest,
	validateMetricsRequestType,
} from "@/helpers/server/platform";

export async function POST(request: Request) {
	const formData = await request.json();
	const timeLimit = formData.timeLimit as TimeLimit;

	const params: MetricParams = {
		timeLimit,
	};

	const validationParam = validateMetricsRequest(
		params,
		validateMetricsRequestType.COST_BY_APPLICATION
	);

	if (!validationParam.success)
		return Response.json(validationParam.err, {
			status: 400,
		});

	const res: any = await getCostByApplication(params);
	return Response.json(res);
}
