import OpenLitHelper from "@/helpers/server/openlit";
import { Cohere, CohereClient } from "cohere-ai";
import { omit } from "lodash";

export interface CohereProviderEvaluation {
	prompt: string;
	type: "chat";
	token?: string;
	model: string;
	temperature: number;
	p: number;
}

export default class CohereProvider {
	private static async chatCompletion(params: any) {
		const cohere = new CohereClient({
			token: params.token,
		});

		const [err, updatedParams] = await this.transformParams(params);
		if (err) throw new Error(err);

		return await cohere.chat(omit(updatedParams, ["token"]) as any);
	}

	private static async transformParams(params: CohereProviderEvaluation) {
		const updatedParams: any = {};
		updatedParams.token = params.token || process.env.COHERE_TOKEN;
		updatedParams.message = params.prompt;
		updatedParams.stream = false;
		updatedParams.promptTruncation = Cohere.ChatRequestPromptTruncation.Off;
		updatedParams.temperature = params.temperature;
		updatedParams.model = params.model;
		updatedParams.p = params.p;

		return [null, updatedParams];
	}

	static async evaluate(params: CohereProviderEvaluation) {
		switch (params.type) {
			case "chat":
				const start = performance.now();
				const response = await this.chatCompletion(params);
				const end = performance.now();
				const updatedResponse: any = response;
				updatedResponse.evaluationData = {
					cost: OpenLitHelper.getChatModelCost(
						params.model,
						response.meta?.billedUnits?.inputTokens || 0,
						response.meta?.billedUnits?.outputTokens || 0
					),
					promptTokens: response.meta?.billedUnits?.inputTokens || 0,
					completionTokens: response.meta?.billedUnits?.outputTokens || 0,
					responseTime: ((end - start) / 1000).toFixed(4),
					model: params.model,
					prompt: params.prompt,
					// @ts-expect-error Type key not defined
					response: response.chatHistory?.[1]?.message || "",
				};
				return updatedResponse;
			default:
				throw new Error("No supported operation");
		}
	}
}
