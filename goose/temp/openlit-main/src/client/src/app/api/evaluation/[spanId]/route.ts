import { getEvaluationsForSpanId, setEvaluationsForSpanId } from "@/lib/platform/evaluation";
import { NextRequest } from "next/server";

export async function GET(
	_: NextRequest,
	{ params }: { params: { spanId: string } }
) {
	const { spanId } = params;

	const res: any = await getEvaluationsForSpanId(spanId);
	return Response.json(res);
}

export async function POST(
	request: Request,
	{ params }: { params: { spanId: string } }
) {

	const { spanId } = params;

	const res: any = await setEvaluationsForSpanId(spanId);
	return Response.json(res);
}
