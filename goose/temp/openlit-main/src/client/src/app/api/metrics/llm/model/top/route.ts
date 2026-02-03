import { ModelMetricParams, getTopModels } from "@/lib/platform/llm/model";
import {
	validateMetricsRequest,
	validateMetricsRequestType,
} from "@/helpers/server/platform";
import { TimeLimit } from "@/lib/platform/common";

export async function POST(request: Request) {
	const formData = await request.json();
	const timeLimit = formData.timeLimit as TimeLimit;

	const params: ModelMetricParams = {
		timeLimit,
		top: formData.top || 3,
	};

	const validationParam = validateMetricsRequest(
		params,
		validateMetricsRequestType.TOP_MODELS
	);

	if (!validationParam.success)
		return Response.json(validationParam.err, {
			status: 400,
		});

	const res: any = await getTopModels(params);
	return Response.json(res);
}
