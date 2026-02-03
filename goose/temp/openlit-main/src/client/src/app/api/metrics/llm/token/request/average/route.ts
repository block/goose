import {
	type TokenParams,
	getAverageTokensPerRequest,
} from "@/lib/platform/llm/token";
import {
	validateMetricsRequest,
	validateMetricsRequestType,
} from "@/helpers/server/platform";
import { TimeLimit } from "@/lib/platform/common";

export async function POST(request: Request) {
	const formData = await request.json();
	const timeLimit = formData.timeLimit as TimeLimit;

	const params: TokenParams = {
		timeLimit,
		type: formData.type,
	};

	const validationParam = validateMetricsRequest(
		params,
		validateMetricsRequestType.AVERAGE_REQUEST_TOKEN
	);

	if (!validationParam.success)
		return Response.json(validationParam.err, {
			status: 400,
		});

	const res: any = await getAverageTokensPerRequest(params);
	return Response.json(res);
}
