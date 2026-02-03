import { MetricParams, TimeLimit } from "@/lib/platform/common";
import { getRequests } from "@/lib/platform/request";
import {
	validateMetricsRequest,
	validateMetricsRequestType,
} from "@/helpers/server/platform";

export async function POST(request: Request) {
	const formData = await request.json();
	const timeLimit = formData.timeLimit as TimeLimit;
	const limit = formData.limit || 10;
	const offset = formData.offset || 0;
	const selectedConfig = formData.selectedConfig || {};
	const sorting = formData.sorting || {};

	const params: MetricParams = {
		timeLimit,
		limit,
		offset,
		selectedConfig,
		sorting,
	};

	const validationParam = validateMetricsRequest(
		params,
		validateMetricsRequestType.GET_ALL
	);

	if (!validationParam.success)
		return Response.json(validationParam.err, {
			status: 400,
		});

	const res: any = await getRequests(params);
	return Response.json(res);
}
