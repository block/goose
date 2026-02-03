import {
	getEvaluationConfig,
	setEvaluationConfig,
} from "@/lib/platform/evaluation/config";
import { EvaluationConfigInput } from "@/types/evaluation";
import { NextRequest } from "next/server";
import asaw from "@/utils/asaw";

export async function GET(_: NextRequest) {
	const res: any = await getEvaluationConfig(undefined, true, false);
	return Response.json(res);
}

export async function POST(request: NextRequest) {
	const formData = await request.json();
	const evaluationConfig: EvaluationConfigInput = {
		id: formData.id,
		provider: formData.provider,
		model: formData.model,
		vaultId: formData.vaultId,
		auto: formData.auto,
		recurringTime: formData.recurringTime || "",
		meta: formData.meta || "{}",
	};

	const [err, data] = await asaw(
		setEvaluationConfig(evaluationConfig, request.nextUrl.origin)
	);

	if (err) {
		return Response.json(err, {
			status: 400,
		});
	}

	return Response.json(data);
}
