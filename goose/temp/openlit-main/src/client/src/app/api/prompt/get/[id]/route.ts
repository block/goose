import { getPromptDetails } from "@/lib/platform/prompt";
import { NextRequest } from "next/server";

export async function GET(request: NextRequest, context: any) {
	const { searchParams } = request.nextUrl;
	const version = searchParams.get("version") || undefined;
	const { id } = context.params || {};

	const res: any = await getPromptDetails(id, { version });
	return Response.json(res);
}
