import OpenLitHelper from "@/helpers/server/openlit";
import { omit } from "lodash";
import OpenAI from "openai";

export interface OpenAIProviderEvaluation {
	api_key?: string;
	prompt: string;
	type: "chat";
	temperature: number;
	top_p: number;
	model: string;
	max_tokens: number;
}

export default class OpenAIProvider {
	private static async chatCompletion(params: any) {
		const openai = new OpenAI({
			apiKey: params.api_key,
		});

		const [err, updatedParams] = await this.transformParams(params);
		if (err) throw new Error(err);

		return await openai.chat.completions.create(
			omit(updatedParams, ["api_key"]) as any
		);
	}

	private static async transformParams(params: OpenAIProviderEvaluation) {
		const updatedParams: any = {};
		updatedParams.api_key = params.api_key || process.env.OPENAI_API_KEY;
		updatedParams.messages = [{ role: "user", content: params.prompt }];
		updatedParams.max_tokens = params.max_tokens;
		updatedParams.model = params.model;
		updatedParams.temperature = params.temperature;
		updatedParams.top_p = params.top_p;
		updatedParams.stream = false;
		return [null, updatedParams];
	}

	static async evaluate(params: OpenAIProviderEvaluation) {
		switch (params.type) {
			case "chat":
				const start = performance.now();
				const response = await this.chatCompletion(params);
				const end = performance.now();
				const updatedResponse: any = response;
				updatedResponse.evaluationData = {
					cost: OpenLitHelper.getChatModelCost(
						response.model,
						response.usage?.prompt_tokens || 0,
						response.usage?.completion_tokens || 0
					),
					promptTokens: response.usage?.prompt_tokens || 0,
					completionTokens: response.usage?.completion_tokens || 0,
					responseTime: ((end - start) / 1000).toFixed(4),
					model: params.model,
					prompt: params.prompt,
					response: response.choices[0].message.content,
				};
				return updatedResponse;
			default:
				throw new Error("No supported operation");
		}
	}
}
